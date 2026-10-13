use crate::{Diagnostic, Span};

pub(crate) struct ParsedFunction {
    pub name: String,
    pub parameters: Vec<String>,
    pub parameter_types: Vec<Option<crate::ParameterType>>,
    pub result_type: Option<crate::ReturnType>,
    pub statements: Vec<Statement>,
    pub body_span: Span,
}

pub(crate) enum Statement {
    Local(LocalDeclaration),
    Println {
        argument: Expression,
        span: Span,
    },
    BaseInitialize {
        span: Span,
    },
    Assign {
        name: String,
        value: Expression,
        span: Span,
    },
    Call {
        module: String,
        member: String,
        arguments: Vec<Expression>,
        span: Span,
    },
    ValueCall {
        expression: Expression,
        span: Span,
    },
    NativeModuleCall {
        module: String,
        member: String,
        arguments: Vec<Expression>,
        span: Span,
    },
    Return {
        value: Option<Expression>,
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
            | Self::Call { span, .. }
            | Self::ValueCall { span, .. }
            | Self::NativeModuleCall { span, .. }
            | Self::If { span, .. }
            | Self::Break { span }
            | Self::Continue { span }
            | Self::While { span, .. } => *span,
            Self::BaseInitialize { span } => *span,
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
    ArrayLiteral(Vec<Expression>),
    ArrayRead {
        array: Box<Expression>,
        index: Box<Expression>,
    },
    // Builder-only read of an evaluated temporary; never parsed from source.
    Temporary(crate::LocalId),
    Logical {
        and: bool,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Char(u32),
    Negate(Box<Expression>),
    Not(Box<Expression>),
    Null,
    Boolean(bool),
    Integer(i32),
    String(String),
    Local(String),
    ModuleCall {
        module: String,
        member: String,
        arguments: Vec<Expression>,
    },
    ValueCall {
        receiver: Box<Expression>,
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

impl Expression {
    /// Report whether this tree needs control flow or staged array construction.
    pub(crate) fn needs_materialization(&self) -> bool {
        match &self.kind {
            ExpressionKind::Logical { .. } | ExpressionKind::ArrayLiteral(_) => true,
            ExpressionKind::Not(operand) | ExpressionKind::Negate(operand) => {
                operand.needs_materialization()
            }
            ExpressionKind::Binary { left, right, .. }
            | ExpressionKind::Compare { left, right, .. } => {
                left.needs_materialization() || right.needs_materialization()
            }
            ExpressionKind::ValueCall {
                receiver,
                arguments,
                ..
            } => {
                receiver.needs_materialization()
                    || arguments.iter().any(Self::needs_materialization)
            }
            ExpressionKind::ArrayRead { array, index } => {
                array.needs_materialization() || index.needs_materialization()
            }
            ExpressionKind::ModuleCall { arguments, .. } => {
                arguments.iter().any(Self::needs_materialization)
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind<'a> {
    Char(u32),
    Word(&'a str),
    Integer(&'a str),
    String(&'a str),
    Punctuation(char),
    EqualEqual,
    AndAnd,
    OrOr,
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
    names: crate::names::Names,
    native_base: bool,
}

impl<'a> Parser<'a> {
    /// Parse the supported Array element annotation after its canonical path.
    /// Only explicit Char elements are accepted; raw and nullable arrays are not.
    fn char_array_annotation(&mut self) -> Result<(), Diagnostic> {
        self.expect(Kind::Punctuation('<'), "'<' and an Array element type")?;
        let at = self.offset;
        if self.path()? != "Toybox.Lang.Char" {
            return Err(self.error(at, "only Array<Toybox.Lang.Char> is supported"));
        }
        self.expect(Kind::Punctuation('>'), "'>'")?;
        Ok(())
    }
    fn path(&mut self) -> Result<String, Diagnostic> {
        let token = self.identifier()?;
        let Kind::Word(name) = token.kind else {
            unreachable!()
        };
        let mut path = name.to_owned();
        while self.peek()? == Kind::Punctuation('.') {
            self.token()?;
            let token = self.identifier()?;
            let Kind::Word(name) = token.kind else {
                unreachable!()
            };
            path.push('.');
            path.push_str(name);
        }
        Ok(path)
    }

    fn imports(&mut self) -> Result<(), Diagnostic> {
        while matches!(self.peek()?, Kind::Word("using" | "import")) {
            let token = self.token()?;
            let import = token.kind == Kind::Word("import");
            let path = self.path()?;
            let alias = if self.peek()? == Kind::Word("as") {
                if import {
                    return Err(self.error(token.span.start, "import aliases are unsupported"));
                }
                self.token()?;
                let token = self.identifier()?;
                let Kind::Word(name) = token.kind else {
                    unreachable!()
                };
                Some(name)
            } else {
                None
            };
            self.expect(Kind::Punctuation(';'), "';'")?;
            self.names
                .declare(&path, alias, import)
                .map_err(|message| self.error(token.span.start, message))?;
        }
        Ok(())
    }

    fn reject_binding(&self, token: &Token<'_>) -> Result<(), Diagnostic> {
        if let Kind::Word(name) = token.kind {
            if self.names.bound(name) || (self.native_base && name == "AppBase") {
                return Err(self.error(
                    token.span.start,
                    format!("declaration shadows native binding {name:?}"),
                ));
            }
        }
        Ok(())
    }
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
            Some('\'') => {
                self.offset += 1;
                let mut value = self.source[self.offset..]
                    .chars()
                    .next()
                    .ok_or_else(|| self.error(start, "unterminated Char literal"))?;
                self.offset += value.len_utf8();
                if value == '\\' {
                    let escape = self.source[self.offset..]
                        .chars()
                        .next()
                        .ok_or_else(|| self.error(start, "unfinished Char escape"))?;
                    self.offset += escape.len_utf8();
                    value = match escape {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '\'' => '\'',
                        '\\' => '\\',
                        _ => return Err(self.error(start, "unsupported source Char escape")),
                    };
                } else if matches!(value, '\'' | '\n' | '\r') {
                    return Err(self.error(
                        start,
                        "Char literals require one character without raw newlines",
                    ));
                }
                if u32::from(value) > 0xffff {
                    return Err(self.error(
                        start,
                        "source Char literals currently require a BMP character",
                    ));
                }
                if !self.source[self.offset..].starts_with('\'') {
                    return Err(self.error(start, "Char literals require exactly one character"));
                }
                self.offset += 1;
                Kind::Char(u32::from(value))
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
            Some('-') if rest.starts_with("--") => {
                return Err(self.error(start, "decrement operators are unsupported"));
            }
            Some('&') if rest.starts_with("&&") => {
                self.offset += 2;
                Kind::AndAnd
            }
            Some('|') if rest.starts_with("||") => {
                self.offset += 2;
                Kind::OrOr
            }
            Some('=') if rest.starts_with("==") => {
                self.offset += 2;
                Kind::EqualEqual
            }
            Some(
                c @ ('(' | ')' | '{' | '}' | '[' | ']' | ';' | '+' | '-' | '=' | '.' | ',' | '>'
                | '<' | '!'),
            ) => {
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
                    "self"
                        | "function"
                        | "module"
                        | "class"
                        | "extends"
                        | "using"
                        | "import"
                        | "as"
                        | "return"
                        | "var"
                        | "if"
                        | "else"
                        | "while"
                        | "break"
                        | "continue"
                        | "true"
                        | "false"
                        | "null"
                ) =>
            {
                Ok(token)
            }
            _ => Err(self.error(token.span.start, "expected a non-reserved identifier")),
        }
    }
    fn expression(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        self.logical(depth, false)
    }
    fn logical(&mut self, depth: usize, and: bool) -> Result<Expression, Diagnostic> {
        let operand = |parser: &mut Self| {
            if and {
                parser.comparison(depth)
            } else {
                parser.logical(depth, true)
            }
        };
        let mut left = operand(self)?;
        while self.peek()? == if and { Kind::AndAnd } else { Kind::OrOr } {
            self.token()?;
            let right = operand(self)?;
            let span = Span {
                start: left.span.start,
                end: right.span.end,
            };
            left = self.node(
                ExpressionKind::Logical {
                    and,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            )?;
        }
        Ok(left)
    }
    fn comparison(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
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
        let mut left = self.unary(depth)?;
        loop {
            let kind = match self.peek()? {
                Kind::Punctuation('+') => crate::BinaryOp::Add,
                Kind::Punctuation('-') => crate::BinaryOp::Subtract,
                _ => break,
            };
            self.token()?;
            let right = self.unary(depth)?;
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
    fn unary(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        if self.peek()? == Kind::Punctuation('!') {
            let sign = self.token()?;
            if depth >= 64 {
                return Err(self.error(sign.span.start, "expression nesting exceeds 64 levels"));
            }
            let operand = self.unary(depth + 1)?;
            let span = Span {
                start: sign.span.start,
                end: operand.span.end,
            };
            return self.node(ExpressionKind::Not(Box::new(operand)), span);
        }
        if self.peek()? != Kind::Punctuation('-') {
            return self.primary(depth);
        }
        let sign = self.token()?;
        if depth >= 64 {
            return Err(self.error(sign.span.start, "expression nesting exceeds 64 levels"));
        }
        if let Kind::Integer(digits) = self.peek()? {
            let magnitude = self.token()?;
            let value = digits
                .parse::<i64>()
                .ok()
                .and_then(|value| i32::try_from(-value).ok())
                .ok_or_else(|| {
                    self.error(
                        sign.span.start,
                        "Number literal exceeds signed 32-bit range",
                    )
                })?;
            if self.peek()? == Kind::Punctuation('.') {
                return Err(self.error(
                    sign.span.start,
                    "parenthesize a negative literal before a member call",
                ));
            }
            return self.node(
                ExpressionKind::Integer(value),
                Span {
                    start: sign.span.start,
                    end: magnitude.span.end,
                },
            );
        }
        if self.peek()? == Kind::Punctuation('-') {
            return Err(self.error(sign.span.start, "parenthesize repeated negation"));
        }
        let operand = self.primary(depth + 1)?;
        let span = Span {
            start: sign.span.start,
            end: operand.span.end,
        };
        self.node(ExpressionKind::Negate(Box::new(operand)), span)
    }
    fn primary(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let mut expression = if self.peek()? == Kind::Punctuation('[') {
            let open = self.token()?;
            if depth >= 64 {
                return Err(self.error(open.span.start, "expression nesting exceeds 64 levels"));
            }
            if self.peek()? == Kind::Punctuation(']') {
                return Err(self.error(
                    open.span.start,
                    "empty array literals require element-type inference, which is not supported",
                ));
            }
            let mut elements = vec![self.expression(depth + 1)?];
            while self.peek()? == Kind::Punctuation(',') {
                self.token()?;
                elements.push(self.expression(depth + 1)?);
            }
            let close = self.expect(Kind::Punctuation(']'), "']'")?;
            self.node(
                ExpressionKind::ArrayLiteral(elements),
                Span {
                    start: open.span.start,
                    end: close.span.end,
                },
            )?
        } else if self.peek()? == Kind::Punctuation('(') {
            let open = self.token()?;
            if depth >= 64 {
                return Err(self.error(open.span.start, "expression nesting exceeds 64 levels"));
            }
            let mut expression = self.expression(depth + 1)?;
            let close = self.expect(Kind::Punctuation(')'), "')'")?;
            expression.span = Span {
                start: open.span.start,
                end: close.span.end,
            };
            expression
        } else {
            self.atom(depth)?
        };
        let mut call_depth = depth;
        while matches!(
            self.peek()?,
            Kind::Punctuation('.') | Kind::Punctuation('[')
        ) {
            if self.peek()? == Kind::Punctuation('[') {
                let open = self.token()?;
                if call_depth >= 64 {
                    return Err(self.error(open.span.start, "expression nesting exceeds 64 levels"));
                }
                let index = self.expression(call_depth + 1)?;
                let end = self.expect(Kind::Punctuation(']'), "']'")?.span.end;
                let start = expression.span.start;
                call_depth += 1;
                expression = self.node(
                    ExpressionKind::ArrayRead {
                        array: Box::new(expression),
                        index: Box::new(index),
                    },
                    Span { start, end },
                )?;
                continue;
            }
            self.token()?;
            let token = self.identifier()?;
            let Kind::Word(member) = token.kind else {
                unreachable!()
            };
            let start = expression.span.start;
            let (arguments, end) = self.call_arguments(start, call_depth)?;
            call_depth += 1;
            expression = self.node(
                ExpressionKind::ValueCall {
                    receiver: Box::new(expression),
                    member: member.into(),
                    arguments,
                },
                Span { start, end },
            )?;
        }
        Ok(expression)
    }

    /// Parse bounded positional arguments and return the closing source offset.
    fn call_arguments(
        &mut self,
        start: usize,
        depth: usize,
    ) -> Result<(Vec<Expression>, usize), Diagnostic> {
        self.expect(
            Kind::Punctuation('('),
            "'(' (member calls require parentheses)",
        )?;
        if depth >= 64 {
            return Err(self.error(start, "expression nesting exceeds 64 levels"));
        }
        let mut arguments = Vec::new();
        if self.peek()? != Kind::Punctuation(')') {
            loop {
                if arguments.len() >= crate::MAX_PARAMETERS {
                    return Err(self.error(start, "call exceeds 254-argument limit"));
                }
                arguments.push(self.expression(depth + 1)?);
                if self.peek()? != Kind::Punctuation(',') {
                    break;
                }
                self.token()?;
            }
        }
        let end = self.expect(Kind::Punctuation(')'), "')'")?.span.end;
        Ok((arguments, end))
    }

    fn atom(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let token = self.token()?;
        match token.kind {
            Kind::Char(value) => self.node(ExpressionKind::Char(value), token.span),
            Kind::String(literal) => {
                let fields = ciq_prg::symbols::source::tokens(literal)
                    .map_err(|error| self.error(token.span.start, error.to_string()))?;
                self.node(ExpressionKind::String(fields[0][1..].into()), token.span)
            }
            Kind::Integer(digits) => {
                let value = digits.parse::<i32>().map_err(|_| {
                    self.error(
                        token.span.start,
                        "Number literal exceeds signed 32-bit range",
                    )
                })?;
                self.node(ExpressionKind::Integer(value), token.span)
            }
            Kind::Word("null") => self.node(ExpressionKind::Null, token.span),
            Kind::Word("true") => self.node(ExpressionKind::Boolean(true), token.span),
            Kind::Word("false") => self.node(ExpressionKind::Boolean(false), token.span),
            Kind::Word(name)
                if !matches!(
                    name,
                    "function"
                        | "module"
                        | "class"
                        | "extends"
                        | "using"
                        | "import"
                        | "as"
                        | "return"
                        | "var"
                        | "if"
                        | "else"
                        | "while"
                        | "break"
                        | "continue"
                        | "true"
                        | "false"
                        | "null"
                ) =>
            {
                if self.peek()? == Kind::Punctuation('.') {
                    self.token()?;
                    let member = self.identifier()?;
                    let Kind::Word(member) = member.kind else {
                        unreachable!()
                    };
                    let mut path = format!("{name}.{member}");
                    while self.peek()? == Kind::Punctuation('.') {
                        self.token()?;
                        let token = self.identifier()?;
                        let Kind::Word(part) = token.kind else {
                            unreachable!()
                        };
                        path.push('.');
                        path.push_str(part);
                    }
                    let (module, member) = path.rsplit_once('.').unwrap();
                    let module = if let Some(native) = self.names.module(module) {
                        native.name()
                    } else if self.native_base
                        && (module == "AppBase" || self.names.app_base(module))
                    {
                        "Application.AppBase"
                    } else if module.contains('.') {
                        return Err(
                            self.error(token.span.start, "unsupported qualified module path")
                        );
                    } else {
                        module
                    };
                    let (arguments, end) = self.call_arguments(token.span.start, depth)?;
                    self.node(
                        ExpressionKind::ModuleCall {
                            module: module.into(),
                            member: member.into(),
                            arguments,
                        },
                        Span {
                            start: token.span.start,
                            end,
                        },
                    )
                } else if name == "self" {
                    Err(self.error(token.span.start, "self is supported only in receiver calls"))
                } else {
                    self.node(ExpressionKind::Local(name.into()), token.span)
                }
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
                    self.reject_binding(&token)?;
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
                    let value = if self.peek()? == Kind::Punctuation(';') {
                        None
                    } else {
                        Some(self.expression(0)?)
                    };
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
                        if matches!(expression.kind, ExpressionKind::ValueCall { .. }) {
                            Statement::ValueCall {
                                expression,
                                span: Span { start, end },
                            }
                        } else {
                            let ExpressionKind::ModuleCall {
                                module,
                                member,
                                mut arguments,
                            } = expression.kind
                            else {
                                return Err(self.error(start, "only assignment and calls are supported as expression statements"));
                            };
                            if module == "Application.AppBase" {
                                if member != "initialize" || !arguments.is_empty() {
                                    return Err(self.error(
                                        start,
                                        "only a zero-argument native base initialize call is supported",
                                    ));
                                }
                                Statement::BaseInitialize {
                                    span: Span { start, end },
                                }
                            } else if module == "System" {
                                if member == "println" {
                                    if arguments.len() != 1 {
                                        return Err(self.error(
                                            start,
                                            "expected System.println with one argument",
                                        ));
                                    }
                                    Statement::Println {
                                        argument: arguments.remove(0),
                                        span: Span { start, end },
                                    }
                                } else {
                                    Statement::NativeModuleCall {
                                        module: "Toybox.System".into(),
                                        member,
                                        arguments,
                                        span: Span { start, end },
                                    }
                                }
                            } else {
                                Statement::Call {
                                    module,
                                    member,
                                    arguments,
                                    span: Span { start, end },
                                }
                            }
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
        names: crate::names::Names::default(),
        native_base: false,
    };
    parser.imports()?;
    let function = parser.function()?;
    parser.expect(Kind::End, "end of input (only one function is supported)")?;
    Ok(function)
}

impl Parser<'_> {
    fn function(&mut self) -> Result<ParsedFunction, Diagnostic> {
        self.expressions = 0;
        self.expect(Kind::Word("function"), "'function'")?;
        let name = self.identifier()?;
        self.reject_binding(&name)?;
        let Kind::Word(identifier) = name.kind else {
            unreachable!()
        };
        self.expect(Kind::Punctuation('('), "'('")?;
        let mut parameters = Vec::new();
        let mut parameter_types = Vec::new();
        if self.peek()? != Kind::Punctuation(')') {
            loop {
                let parameter = self.identifier()?;
                self.reject_binding(&parameter)?;
                let Kind::Word(name) = parameter.kind else {
                    unreachable!()
                };
                if parameters.len() >= crate::MAX_PARAMETERS {
                    return Err(
                        self.error(parameter.span.start, "function exceeds 254-parameter limit")
                    );
                }
                if parameters.iter().any(|p| p == name) {
                    return Err(self.error(
                        parameter.span.start,
                        format!("duplicate parameter {name:?}"),
                    ));
                }
                parameters.push(name.to_owned());
                parameter_types.push(if self.peek()? == Kind::Word("as") {
                    self.token()?;
                    let at = self.offset;
                    let ty = match self.path()?.as_str() {
                        "Toybox.Lang.Number" => crate::ParameterType::Integer,
                        "Toybox.Lang.String" => crate::ParameterType::String,
                        "Toybox.Lang.Boolean" => crate::ParameterType::Boolean,
                        "Toybox.Lang.Char" => crate::ParameterType::Char,
                        "Toybox.Lang.Array" => {
                            self.char_array_annotation()?;
                            crate::ParameterType::CharArray
                        }
                        _ => return Err(self.error(at, "unsupported parameter type annotation")),
                    };
                    Some(if self.peek()? == Kind::Word("or") {
                        self.token()?;
                        self.expect(Kind::Word("Null"), "'Null'")?;
                        match ty {
                            crate::ParameterType::Integer => crate::ParameterType::NullableInteger,
                            crate::ParameterType::String => crate::ParameterType::NullableString,
                            _ => return Err(self.error(at, "unsupported nullable parameter type")),
                        }
                    } else {
                        ty
                    })
                } else {
                    None
                });
                if self.peek()? != Kind::Punctuation(',') {
                    break;
                }
                self.token()?;
            }
        }
        self.expect(Kind::Punctuation(')'), "')'")?;
        let result_type = if self.peek()? == Kind::Word("as") {
            self.token()?;
            let at = self.offset;
            let ty = match self.path()?.as_str() {
                "Toybox.Lang.Number" => crate::ReturnType::Integer,
                "Toybox.Lang.String" => crate::ReturnType::String,
                "Toybox.Lang.Boolean" => crate::ReturnType::Boolean,
                "Toybox.Lang.Char" => crate::ReturnType::Char,
                "Toybox.Lang.Array" => {
                    self.char_array_annotation()?;
                    crate::ReturnType::CharArray
                }
                "Void" => crate::ReturnType::Void,
                _ => return Err(self.error(at, "unsupported result type annotation")),
            };
            Some(if self.peek()? == Kind::Word("or") {
                self.token()?;
                self.expect(Kind::Word("Null"), "'Null'")?;
                match ty {
                    crate::ReturnType::Integer => crate::ReturnType::NullableInteger,
                    crate::ReturnType::String => crate::ReturnType::NullableString,
                    _ => return Err(self.error(at, "unsupported nullable result type")),
                }
            } else {
                ty
            })
        } else {
            None
        };
        let (statements, body_span) = self.block(0)?;
        Ok(ParsedFunction {
            name: identifier.to_owned(),
            parameters,
            parameter_types,
            result_type,
            statements,
            body_span,
        })
    }
}

pub(crate) struct ParsedUnit {
    pub functions: Vec<(Option<String>, ParsedFunction)>,
    pub classes: Vec<ParsedClass>,
}
pub(crate) struct ParsedClass {
    pub name: String,
    pub methods: Vec<ParsedFunction>,
}

/// Parse global/module functions and AppBase-derived class declarations.
/// Collects source ownership without binding calls or selecting VM instructions.
pub(crate) fn declarations(source: &str) -> Result<ParsedUnit, Diagnostic> {
    let mut parser = Parser {
        source,
        offset: 0,
        lookahead: None,
        expressions: 0,
        names: crate::names::Names::default(),
        native_base: false,
    };
    let mut unit = ParsedUnit {
        functions: Vec::new(),
        classes: Vec::new(),
    };
    let mut count = 0;
    parser.imports()?;
    while parser.peek()? != Kind::End {
        if matches!(parser.peek()?, Kind::Word("module" | "class")) {
            let is_class = parser.token()?.kind == Kind::Word("class");
            let token = parser.identifier()?;
            parser.reject_binding(&token)?;
            let Kind::Word(name) = token.kind else {
                unreachable!()
            };
            if is_class {
                parser.expect(Kind::Word("extends"), "'extends Application.AppBase'")?;
                let base = parser.path()?;
                if !parser.names.app_base(&base) {
                    return Err(parser.error(
                        token.span.start,
                        format!("unsupported or unresolved native superclass {base:?}"),
                    ));
                }
            } else if name == "System" {
                return Err(parser.error(token.span.start, "System is reserved for SDK intrinsics"));
            }
            parser.expect(Kind::Punctuation('{'), "'{'")?;
            let outer_names = parser.names.clone();
            parser.native_base = is_class;
            parser.imports()?;
            let mut methods = Vec::new();
            while parser.peek()? != Kind::Punctuation('}') {
                methods.push(parser.function()?);
                count += 1;
                if count > crate::unit::MAX_FUNCTIONS {
                    return Err(parser.error(
                        token.span.start,
                        "compilation unit exceeds 256-function limit",
                    ));
                }
            }
            parser.token()?;
            parser.names = outer_names;
            parser.native_base = false;
            if methods.is_empty() {
                return Err(parser.error(token.span.start, "empty modules/classes are unsupported"));
            }
            if is_class {
                unit.classes.push(ParsedClass {
                    name: name.into(),
                    methods,
                });
            } else {
                unit.functions
                    .extend(methods.into_iter().map(|f| (Some(name.into()), f)));
            }
        } else {
            unit.functions.push((None, parser.function()?));
            count += 1;
        }
        if count > crate::unit::MAX_FUNCTIONS {
            return Err(parser.error(parser.offset, "compilation unit exceeds 256-function limit"));
        }
    }
    Ok(unit)
}
