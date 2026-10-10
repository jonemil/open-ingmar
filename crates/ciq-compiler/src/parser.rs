use crate::{Diagnostic, Span};

pub(crate) struct ParsedFunction {
    pub name: String,
    pub parameters: Vec<String>,
    pub statements: Vec<Statement>,
    pub body_span: Span,
}

pub(crate) enum Statement {
    Local(LocalDeclaration),
    Println {
        argument: Expression,
        span: Span,
    },
    Assign {
        name: String,
        value: Expression,
        span: Span,
    },
    Return {
        value: Expression,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
        span: Span,
    },
    If {
        condition: Expression,
        on_true: Vec<Statement>,
        on_false: Vec<Statement>,
        span: Span,
    },
}

impl Statement {
    pub fn span(&self) -> Span {
        match self {
            Self::Local(local) => local.span,
            Self::Println { span, .. }
            | Self::Assign { span, .. }
            | Self::Return { span, .. }
            | Self::If { span, .. }
            | Self::Break { span }
            | Self::Continue { span }
            | Self::While { span, .. } => *span,
        }
    }
}

pub(crate) struct LocalDeclaration {
    pub name: String,
    pub initializer: Expression,
    pub span: Span,
}

pub(crate) struct Expression {
    pub kind: ExpressionKind,
    pub span: Span,
}
pub(crate) enum ExpressionKind {
    Integer(i32),
    String(String),
    Local(String),
    ModuleCall {
        module: String,
        member: String,
        arguments: Vec<Expression>,
    },
    Binary {
        kind: crate::BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Compare {
        kind: crate::ComparisonOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind<'a> {
    Word(&'a str),
    Integer(&'a str),
    String(&'a str),
    Punctuation(char),
    EqualEqual,
    End,
}

struct Token<'a> {
    kind: Kind<'a>,
    span: Span,
}

struct Parser<'a> {
    source: &'a str,
    offset: usize,
    lookahead: Option<Token<'a>>,
    expressions: usize,
}

impl<'a> Parser<'a> {
    fn error(&self, offset: usize, message: impl Into<String>) -> Diagnostic {
        Diagnostic::at(self.source, offset, message)
    }

    fn read_token(&mut self) -> Result<Token<'a>, Diagnostic> {
        loop {
            while let Some(c) = self.source[self.offset..].chars().next() {
                if !c.is_whitespace() {
                    break;
                }
                self.offset += c.len_utf8();
            }
            let rest = &self.source[self.offset..];
            if rest.starts_with("//") {
                self.offset += rest.find('\n').unwrap_or(rest.len());
            } else if let Some(comment) = rest.strip_prefix("/*") {
                let end = comment
                    .find("*/")
                    .ok_or_else(|| self.error(self.offset, "unterminated block comment"))?;
                self.offset += end + 4;
            } else {
                break;
            }
        }
        let start = self.offset;
        let rest = &self.source[start..];
        let kind = match rest.chars().next() {
            None => Kind::End,
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {
                let length = rest
                    .bytes()
                    .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
                    .count();
                self.offset += length;
                Kind::Word(&rest[..length])
            }
            Some('"') => {
                self.offset += 1;
                loop {
                    let c = self.source[self.offset..]
                        .chars()
                        .next()
                        .ok_or_else(|| self.error(start, "unterminated string literal"))?;
                    self.offset += c.len_utf8();
                    if c == '"' {
                        break;
                    }
                    if c == '\n' || c == '\r' {
                        return Err(
                            self.error(start, "string literals cannot contain raw newlines")
                        );
                    }
                    if c == '\\' {
                        let escaped = self.source[self.offset..]
                            .chars()
                            .next()
                            .ok_or_else(|| self.error(start, "unfinished string escape"))?;
                        if !matches!(escaped, 'n' | 'r' | 't' | '"' | '\\') {
                            return Err(self.error(self.offset, "unsupported source string escape"));
                        }
                        self.offset += escaped.len_utf8();
                    }
                }
                Kind::String(&self.source[start..self.offset])
            }
            Some(c) if c.is_ascii_digit() => {
                let length = rest.bytes().take_while(u8::is_ascii_digit).count();
                self.offset += length;
                Kind::Integer(&rest[..length])
            }
            Some('=') if rest.starts_with("==") => {
                self.offset += 2;
                Kind::EqualEqual
            }
            Some(c @ ('(' | ')' | '{' | '}' | ';' | '+' | '-' | '=' | '.' | ',' | '>')) => {
                self.offset += 1;
                Kind::Punctuation(c)
            }
            Some(c) => return Err(self.error(start, format!("unsupported character {c:?}"))),
        };
        Ok(Token {
            kind,
            span: Span {
                start,
                end: self.offset,
            },
        })
    }

    fn token(&mut self) -> Result<Token<'a>, Diagnostic> {
        if let Some(token) = self.lookahead.take() {
            Ok(token)
        } else {
            self.read_token()
        }
    }
    fn peek(&mut self) -> Result<Kind<'a>, Diagnostic> {
        if self.lookahead.is_none() {
            self.lookahead = Some(self.read_token()?);
        }
        Ok(self.lookahead.as_ref().unwrap().kind)
    }
    fn node(&mut self, kind: ExpressionKind, span: Span) -> Result<Expression, Diagnostic> {
        self.expressions += 1;
        if self.expressions > crate::MAX_OPERATIONS {
            return Err(self.error(span.start, "expression exceeds 1024-operation limit"));
        }
        Ok(Expression { kind, span })
    }
    fn identifier(&mut self) -> Result<Token<'a>, Diagnostic> {
        let token = self.token()?;
        match token.kind {
            Kind::Word(name)
                if !matches!(
                    name,
                    "function"
                        | "return"
                        | "var"
                        | "if"
                        | "else"
                        | "while"
                        | "break"
                        | "continue"
                        | "true"
                        | "false"
                ) =>
            {
                Ok(token)
            }
            _ => Err(self.error(token.span.start, "expected a non-reserved identifier")),
        }
    }
    fn expression(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let left = self.arithmetic(depth)?;
        let kind = match self.peek()? {
            Kind::EqualEqual => crate::ComparisonOp::Equal,
            Kind::Punctuation('>') => crate::ComparisonOp::Greater,
            _ => return Ok(left),
        };
        self.token()?;
        let right = self.arithmetic(depth)?;
        let span = Span {
            start: left.span.start,
            end: right.span.end,
        };
        self.node(
            ExpressionKind::Compare {
                kind,
                left: Box::new(left),
                right: Box::new(right),
            },
            span,
        )
    }
    fn arithmetic(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let mut left = self.primary(depth)?;
        loop {
            let kind = match self.peek()? {
                Kind::Punctuation('+') => crate::BinaryOp::Add,
                Kind::Punctuation('-') => crate::BinaryOp::Subtract,
                _ => break,
            };
            self.token()?;
            let right = self.primary(depth)?;
            let span = Span {
                start: left.span.start,
                end: right.span.end,
            };
            left = self.node(
                ExpressionKind::Binary {
                    kind,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            )?;
        }
        Ok(left)
    }
    fn primary(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let token = self.token()?;
        match token.kind {
            Kind::String(literal) => {
                let fields = ciq_prg::symbols::source::tokens(literal)
                    .map_err(|error| self.error(token.span.start, error.to_string()))?;
                self.node(ExpressionKind::String(fields[0][1..].into()), token.span)
            }
            Kind::Integer(digits) => {
                let value = digits
                    .parse::<i32>()
                    .ok()
                    .filter(|value| *value <= 127)
                    .ok_or_else(|| {
                        self.error(
                            token.span.start,
                            "supported integer literal range is 0..=127",
                        )
                    })?;
                self.node(ExpressionKind::Integer(value), token.span)
            }
            Kind::Word(name)
                if !matches!(
                    name,
                    "function"
                        | "return"
                        | "var"
                        | "if"
                        | "else"
                        | "while"
                        | "break"
                        | "continue"
                        | "true"
                        | "false"
                ) =>
            {
                if self.peek()? == Kind::Punctuation('.') {
                    self.token()?;
                    let member = self.identifier()?;
                    let Kind::Word(member) = member.kind else {
                        unreachable!()
                    };
                    self.expect(
                        Kind::Punctuation('('),
                        "'(' (only module calls are supported)",
                    )?;
                    if depth >= 64 {
                        return Err(
                            self.error(token.span.start, "expression nesting exceeds 64 levels")
                        );
                    }
                    let mut arguments = Vec::new();
                    if self.peek()? != Kind::Punctuation(')') {
                        loop {
                            if arguments.len() >= crate::MAX_PARAMETERS {
                                return Err(
                                    self.error(token.span.start, "call exceeds 254-argument limit")
                                );
                            }
                            arguments.push(self.expression(depth + 1)?);
                            if self.peek()? != Kind::Punctuation(',') {
                                break;
                            }
                            self.token()?;
                        }
                    }
                    let end = self.expect(Kind::Punctuation(')'), "')'")?.span.end;
                    self.node(
                        ExpressionKind::ModuleCall {
                            module: name.into(),
                            member: member.into(),
                            arguments,
                        },
                        Span {
                            start: token.span.start,
                            end,
                        },
                    )
                } else {
                    self.node(ExpressionKind::Local(name.into()), token.span)
                }
            }
            Kind::Punctuation('(') => {
                if depth >= 64 {
                    return Err(
                        self.error(token.span.start, "expression nesting exceeds 64 levels")
                    );
                }
                let mut expression = self.expression(depth + 1)?;
                let close = self.expect(Kind::Punctuation(')'), "')'")?;
                expression.span = Span {
                    start: token.span.start,
                    end: close.span.end,
                };
                Ok(expression)
            }
            _ => Err(self.error(
                token.span.start,
                "expected an integer, local identifier, or parenthesized expression",
            )),
        }
    }

    /// Parse a braced statement sequence, bounding control-flow nesting and total
    /// source operations before semantic construction.
    fn block(&mut self, depth: usize) -> Result<(Vec<Statement>, Span), Diagnostic> {
        let start = self
            .expect(Kind::Punctuation('{'), "'{' (blocks require braces)")?
            .span
            .start;
        if depth > 64 {
            return Err(self.error(start, "control-flow nesting exceeds 64 levels"));
        }
        let mut statements = Vec::new();
        while self.peek()? != Kind::Punctuation('}') {
            let start = self.lookahead.as_ref().unwrap().span.start;
            // A print's call node or assignment target already counts its sink.
            if matches!(
                self.peek()?,
                Kind::Word("var" | "return" | "if" | "while" | "break" | "continue")
            ) {
                self.expressions += 1;
            }
            if self.expressions > crate::MAX_OPERATIONS {
                return Err(self.error(start, "function exceeds 1024-operation limit"));
            }
            let statement = match self.peek()? {
                Kind::Word("var") => {
                    self.token()?;
                    let token = self.identifier()?;
                    let Kind::Word(name) = token.kind else {
                        unreachable!()
                    };
                    self.expect(
                        Kind::Punctuation('='),
                        "'=' (locals require an initializer)",
                    )?;
                    let initializer = self.expression(0)?;
                    let end = self.expect(Kind::Punctuation(';'), "';'")?.span.end;
                    Statement::Local(LocalDeclaration {
                        name: name.into(),
                        initializer,
                        span: Span { start, end },
                    })
                }
                Kind::Word("return") => {
                    self.token()?;
                    let value = self.expression(0)?;
                    let end = self.expect(Kind::Punctuation(';'), "';'")?.span.end;
                    Statement::Return {
                        value,
                        span: Span { start, end },
                    }
                }
                Kind::Word("break" | "continue") => {
                    let keyword = self.token()?.kind;
                    let end = self.expect(Kind::Punctuation(';'), "';'")?.span.end;
                    if keyword == Kind::Word("break") {
                        Statement::Break {
                            span: Span { start, end },
                        }
                    } else {
                        Statement::Continue {
                            span: Span { start, end },
                        }
                    }
                }
                Kind::Word("while") => {
                    self.token()?;
                    self.expect(Kind::Punctuation('('), "'('")?;
                    let condition = self.expression(0)?;
                    self.expect(Kind::Punctuation(')'), "')'")?;
                    let (body, body_span) = self.block(depth + 1)?;
                    Statement::While {
                        condition,
                        body,
                        span: Span {
                            start,
                            end: body_span.end,
                        },
                    }
                }
                Kind::Word("if") => {
                    self.token()?;
                    self.expect(Kind::Punctuation('('), "'('")?;
                    let condition = self.expression(0)?;
                    self.expect(Kind::Punctuation(')'), "')'")?;
                    let (on_true, true_span) = self.block(depth + 1)?;
                    let (on_false, body_span) = if self.peek()? == Kind::Word("else") {
                        self.token()?;
                        self.block(depth + 1)?
                    } else {
                        (Vec::new(), true_span)
                    };
                    Statement::If {
                        condition,
                        on_true,
                        on_false,
                        span: Span {
                            start,
                            end: body_span.end,
                        },
                    }
                }
                Kind::End => return Err(self.error(start, "expected '}' before end of input")),
                _ => {
                    let expression = self.expression(0)?;
                    if self.peek()? == Kind::Punctuation('=') {
                        let ExpressionKind::Local(name) = expression.kind else {
                            return Err(
                                self.error(start, "assignment target must be a local identifier")
                            );
                        };
                        if self.source[expression.span.start..expression.span.end] != name {
                            return Err(self.error(
                                start,
                                "assignment target must be a bare local identifier",
                            ));
                        }
                        self.token()?;
                        let value = self.expression(0)?;
                        let end = self.expect(Kind::Punctuation(';'), "';'")?.span.end;
                        Statement::Assign {
                            name,
                            value,
                            span: Span { start, end },
                        }
                    } else {
                        let end = self.expect(Kind::Punctuation(';'), "';'")?.span.end;
                        let ExpressionKind::ModuleCall {
                            module,
                            member,
                            mut arguments,
                        } = expression.kind
                        else {
                            return Err(self.error(start, "only assignment and System.println are supported as expression statements"));
                        };
                        if module != "System" || member != "println" || arguments.len() != 1 {
                            return Err(
                                self.error(start, "expected System.println with one argument")
                            );
                        }
                        Statement::Println {
                            argument: arguments.remove(0),
                            span: Span { start, end },
                        }
                    }
                }
            };
            statements.push(statement);
        }
        let end = self.token()?.span.end;
        Ok((statements, Span { start, end }))
    }

    fn expect(&mut self, expected: Kind<'a>, description: &str) -> Result<Token<'a>, Diagnostic> {
        let token = self.token()?;
        if token.kind != expected {
            return Err(self.error(token.span.start, format!("expected {description}")));
        }
        Ok(token)
    }
}

pub(crate) fn parse(source: &str) -> Result<ParsedFunction, Diagnostic> {
    let mut parser = Parser {
        source,
        offset: 0,
        lookahead: None,
        expressions: 0,
    };
    parser.expect(Kind::Word("function"), "'function'")?;
    let name = parser.identifier()?;
    let Kind::Word(identifier) = name.kind else {
        unreachable!()
    };
    parser.expect(Kind::Punctuation('('), "'('")?;
    let mut parameters = Vec::new();
    if parser.peek()? != Kind::Punctuation(')') {
        loop {
            let parameter = parser.identifier()?;
            let Kind::Word(name) = parameter.kind else {
                unreachable!()
            };
            if parameters.len() >= crate::MAX_PARAMETERS {
                return Err(
                    parser.error(parameter.span.start, "function exceeds 254-parameter limit")
                );
            }
            if parameters.iter().any(|p| p == name) {
                return Err(parser.error(
                    parameter.span.start,
                    format!("duplicate parameter {name:?}"),
                ));
            }
            parameters.push(name.to_owned());
            if parser.peek()? != Kind::Punctuation(',') {
                break;
            }
            parser.token()?;
        }
    }
    parser.expect(Kind::Punctuation(')'), "')'")?;
    let (statements, body_span) = parser.block(0)?;
    parser.expect(Kind::End, "end of input (only one function is supported)")?;
    Ok(ParsedFunction {
        name: identifier.to_owned(),
        parameters,
        statements,
        body_span,
    })
}
