use crate::{Diagnostic, Span};

pub(crate) struct ParsedFunction {
    pub name: String,
    pub integer: i32,
    pub integer_span: Span,
    pub return_span: Span,
}

#[derive(Debug, PartialEq, Eq)]
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
}

impl<'a> Parser<'a> {
    fn error(&self, offset: usize, message: impl Into<String>) -> Diagnostic {
        Diagnostic::at(self.source, offset, message)
    }

    fn token(&mut self) -> Result<Token<'a>, Diagnostic> {
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
            Some(c @ ('(' | ')' | '{' | '}' | ';')) => {
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

    fn expect(&mut self, expected: Kind<'a>, description: &str) -> Result<Token<'a>, Diagnostic> {
        let token = self.token()?;
        if token.kind != expected {
            return Err(self.error(token.span.start, format!("expected {description}")));
        }
        Ok(token)
    }
}

pub(crate) fn parse(source: &str) -> Result<ParsedFunction, Diagnostic> {
    let mut parser = Parser { source, offset: 0 };
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
    let integer = parser.token()?;
    let Kind::Integer(digits) = integer.kind else {
        return Err(parser.error(integer.span.start, "expected a decimal integer in 0..=127"));
    };
    let value = digits
        .parse::<i32>()
        .ok()
        .filter(|value| *value <= 127)
        .ok_or_else(|| parser.error(integer.span.start, "supported integer range is 0..=127"))?;
    let semi = parser.expect(Kind::Punctuation(';'), "';' (expressions are unsupported)")?;
    parser.expect(
        Kind::Punctuation('}'),
        "'}' (additional statements are unsupported)",
    )?;
    parser.expect(Kind::End, "end of input (only one function is supported)")?;
    Ok(ParsedFunction {
        name: identifier.to_owned(),
        integer: value,
        integer_span: integer.span,
        return_span: Span {
            start: returned.span.start,
            end: semi.span.end,
        },
    })
}
