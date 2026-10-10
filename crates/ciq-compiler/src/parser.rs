use crate::{Diagnostic, Span};

pub(crate) struct ParsedFunction {
    pub name: String,
    pub expression: Expression,
    pub return_span: Span,
}

pub(crate) struct Expression {
    pub kind: ExpressionKind,
    pub span: Span,
}
pub(crate) enum ExpressionKind {
    Integer(i32),
    Binary {
        kind: crate::BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind<'a> {
    Word(&'a str),
    Integer(&'a str),
    Punctuation(char),
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
            Some(c) if c.is_ascii_digit() => {
                let length = rest.bytes().take_while(u8::is_ascii_digit).count();
                self.offset += length;
                Kind::Integer(&rest[..length])
            }
            Some(c @ ('(' | ')' | '{' | '}' | ';' | '+' | '-')) => {
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
        if self.expressions >= crate::MAX_OPERATIONS {
            return Err(self.error(span.start, "expression exceeds 1024-operation limit"));
        }
        Ok(Expression { kind, span })
    }
    fn expression(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
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
                "expected a decimal integer or parenthesized expression",
            )),
        }
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
    let name = parser.token()?;
    let Kind::Word(identifier) = name.kind else {
        return Err(parser.error(name.span.start, "expected a function identifier"));
    };
    if matches!(identifier, "function" | "return") {
        return Err(parser.error(name.span.start, "reserved word cannot name a function"));
    }
    parser.expect(Kind::Punctuation('('), "'('")?;
    parser.expect(Kind::Punctuation(')'), "')' (parameters are unsupported)")?;
    parser.expect(Kind::Punctuation('{'), "'{'")?;
    let returned = parser.expect(Kind::Word("return"), "'return'")?;
    let expression = parser.expression(0)?;
    let semi = parser.expect(Kind::Punctuation(';'), "';'")?;
    parser.expect(
        Kind::Punctuation('}'),
        "'}' (additional statements are unsupported)",
    )?;
    parser.expect(Kind::End, "end of input (only one function is supported)")?;
    Ok(ParsedFunction {
        name: identifier.to_owned(),
        expression,
        return_span: Span {
            start: returned.span.start,
            end: semi.span.end,
        },
    })
}
