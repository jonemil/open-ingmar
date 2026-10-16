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
    Update {
        target: Expression,
        increment: bool,
        span: Span,
    },
    ArrayCompound {
        target: Expression,
        kind: crate::BinaryOp,
        value: Expression,
        span: Span,
    },
    ArrayAssign {
        array: Expression,
        index: Expression,
        value: Expression,
        span: Span,
    },
    FieldAssign {
        name: String,
        value: Expression,
        span: Span,
    },
    Local(LocalDeclaration),
    Println {
        argument: Expression,
        span: Span,
    },
    BaseInitialize {
        span: Span,
        base: crate::ClassBase,
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
            Self::Update { span, .. } => *span,
            Self::FieldAssign { span, .. }
            | Self::ArrayAssign { span, .. }
            | Self::ArrayCompound { span, .. } => *span,
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
            Self::BaseInitialize { span, .. } => *span,
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
    DictionaryNew,
    DictionaryLiteral(Vec<(Expression, Expression)>),
    Conditional {
        condition: Box<Expression>,
        when_true: Box<Expression>,
        when_false: Box<Expression>,
    },
    // Builder-only incoming value at a typed join; consumed in its join block.
    Joined(crate::ValueId),
    // Builder-only numeric step; source updates are statements, not values.
    Step {
        operand: Box<Expression>,
        increment: bool,
    },
    SelfField(String),
    SourceNew {
        class: String,
        arguments: Vec<Expression>,
    },
    NativeConstant {
        module: String,
        member: String,
        qualifier: String,
    },
    NativeNew {
        class: crate::NativeClass,
        qualifier: String,
    },
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
    Positive(Box<Expression>),
    Negate(Box<Expression>),
    Not(Box<Expression>),
    Complement(Box<Expression>),
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
            ExpressionKind::Conditional { .. }
            | ExpressionKind::Logical { .. }
            | ExpressionKind::ArrayLiteral(_)
            | ExpressionKind::DictionaryNew
            | ExpressionKind::DictionaryLiteral(_) => true,
            ExpressionKind::Step { operand, .. } => operand.needs_materialization(),
            ExpressionKind::Not(operand)
            | ExpressionKind::Complement(operand)
            | ExpressionKind::Positive(operand)
            | ExpressionKind::Negate(operand) => operand.needs_materialization(),
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
            ExpressionKind::ModuleCall { arguments, .. }
            | ExpressionKind::SourceNew { arguments, .. } => {
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
    NotEqual,
    LessEqual,
    GreaterEqual,
    ShiftLeft,
    ShiftRight,
    Compound(crate::BinaryOp),
    Step(bool),
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
    dictionary_entry: bool,
    names: crate::names::Names,
    native_base: Option<crate::ClassBase>,
}

impl<'a> Parser<'a> {
    /// Parse the supported Array element annotation after its canonical path.
    /// Number/Char elements are accepted; raw and nullable arrays are not.
    fn dictionary_annotation(&mut self) -> Result<Option<crate::DictionaryType>, Diagnostic> {
        if self.peek()? != Kind::Punctuation('<') {
            return Ok(None);
        }
        self.token()?;
        let at = self.offset;
        let key = match self.path()?.as_str() {
            "Toybox.Lang.Number" => crate::DictionaryKey::Number,
            "Toybox.Lang.String" => crate::DictionaryKey::String,
            "Toybox.Lang.Boolean" => crate::DictionaryKey::Boolean,
            "Toybox.Lang.Char" => crate::DictionaryKey::Char,
            _ => return Err(self.error(at, "unsupported Dictionary key contract")),
        };
        self.expect(Kind::Punctuation(','), "',' and a Dictionary value type")?;
        let mut value = match self.path()?.as_str() {
            "Toybox.Lang.Number" => crate::DictionaryValue::Number,
            "Toybox.Lang.String" => crate::DictionaryValue::String,
            _ => return Err(self.error(at, "Dictionary values require Number or String")),
        };
        if self.peek()? == Kind::Word("or") {
            self.token()?;
            self.expect(Kind::Word("Null"), "'Null'")?;
            value = match value {
                crate::DictionaryValue::Number => crate::DictionaryValue::NullableNumber,
                crate::DictionaryValue::String => crate::DictionaryValue::NullableString,
                _ => unreachable!(),
            };
        }
        self.expect(Kind::Punctuation('>'), "'>'")?;
        Ok(Some(crate::DictionaryType { key, value }))
    }

    fn array_annotation(&mut self) -> Result<crate::ArrayElement, Diagnostic> {
        self.expect(Kind::Punctuation('<'), "'<' and an Array element type")?;
        let at = self.offset;
        let mut element = match self.path()?.as_str() {
            "Toybox.Lang.Char" => crate::ArrayElement::Char,
            "Toybox.Lang.Boolean" => crate::ArrayElement::Boolean,
            "Toybox.Lang.Number" => crate::ArrayElement::Number,
            "Toybox.Lang.String" => crate::ArrayElement::String,
            "Toybox.WatchUi.View" => crate::ArrayElement::Object(crate::NativeClass::View),
            _ => return Err(self.error(at, "unsupported Array element annotation")),
        };
        if self.peek()? == Kind::Word("or") {
            self.token()?;
            self.expect(Kind::Word("Null"), "'Null'")?;
            element = match element {
                crate::ArrayElement::Number => crate::ArrayElement::NullableNumber,
                crate::ArrayElement::String => crate::ArrayElement::NullableString,
                _ => return Err(self.error(at, "nullable Array elements require Number or String")),
            };
        }
        self.expect(Kind::Punctuation('>'), "'>'")?;
        Ok(element)
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
            if self.names.bound(name)
                || (self.native_base == Some(crate::ClassBase::ApplicationAppBase)
                    && name == "AppBase")
                || (self.native_base == Some(crate::ClassBase::WatchUiView) && name == "View")
                || (self.native_base == Some(crate::ClassBase::WatchUiBehaviorDelegate)
                    && name == "BehaviorDelegate")
            {
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
            Some('<' | '>') if rest.starts_with("<<=") || rest.starts_with(">>=") => {
                self.offset += 3;
                Kind::Compound(if rest.starts_with("<<=") {
                    crate::BinaryOp::ShiftLeft
                } else {
                    crate::BinaryOp::ShiftRight
                })
            }
            Some(c @ ('+' | '-' | '*' | '/' | '%' | '&' | '|' | '^'))
                if rest.as_bytes().get(1) == Some(&b'=') =>
            {
                self.offset += 2;
                Kind::Compound(match c {
                    '+' => crate::BinaryOp::Add,
                    '-' => crate::BinaryOp::Subtract,
                    '*' => crate::BinaryOp::Multiply,
                    '/' => crate::BinaryOp::Divide,
                    '%' => crate::BinaryOp::Remainder,
                    '&' => crate::BinaryOp::BitAnd,
                    '|' => crate::BinaryOp::BitOr,
                    '^' => crate::BinaryOp::BitXor,
                    _ => unreachable!("recognized compound operator"),
                })
            }
            Some('-') if rest.starts_with("--") => {
                self.offset += 2;
                Kind::Step(false)
            }
            Some('+') if rest.starts_with("++") => {
                self.offset += 2;
                Kind::Step(true)
            }
            Some('&') if rest.starts_with("&&") => {
                self.offset += 2;
                Kind::AndAnd
            }
            Some('|') if rest.starts_with("||") => {
                self.offset += 2;
                Kind::OrOr
            }
            Some('!') if rest.starts_with("!=") => {
                self.offset += 2;
                Kind::NotEqual
            }
            Some('<') if rest.starts_with("<=") => {
                self.offset += 2;
                Kind::LessEqual
            }
            Some('>') if rest.starts_with(">=") => {
                self.offset += 2;
                Kind::GreaterEqual
            }
            Some('<') if rest.starts_with("<<") => {
                self.offset += 2;
                Kind::ShiftLeft
            }
            Some('>') if rest.starts_with(">>") => {
                self.offset += 2;
                Kind::ShiftRight
            }
            Some('=') if rest.starts_with("==") => {
                self.offset += 2;
                Kind::EqualEqual
            }
            Some(
                c @ ('(' | ')' | '{' | '}' | '[' | ']' | ';' | '+' | '-' | '=' | '.' | ',' | '>'
                | '<' | '!' | '*' | '/' | '%' | '&' | '|' | '^' | '~' | '?' | ':'),
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
                        | "new"
                        | "and"
                        | "or"
                ) =>
            {
                Ok(token)
            }
            _ => Err(self.error(token.span.start, "expected a non-reserved identifier")),
        }
    }
    fn expression(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let condition = self.logical(depth, false)?;
        if self.peek()? == Kind::Punctuation('?') {
            self.conditional(condition, depth)
        } else {
            Ok(condition)
        }
    }

    /// Parse right-associative conditional arms below logical OR precedence.
    fn conditional(
        &mut self,
        condition: Expression,
        depth: usize,
    ) -> Result<Expression, Diagnostic> {
        if depth >= 64 {
            return Err(self.error(condition.span.start, "expression nesting exceeds 64"));
        }
        self.token()?;
        let when_true = self.expression(depth + 1)?;
        self.expect(Kind::Punctuation(':'), "':'")?;
        let when_false = self.expression(depth + 1)?;
        let span = Span {
            start: condition.span.start,
            end: when_false.span.end,
        };
        self.node(
            ExpressionKind::Conditional {
                condition: Box::new(condition),
                when_true: Box::new(when_true),
                when_false: Box::new(when_false),
            },
            span,
        )
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
        while matches!(
            (and, self.peek()?),
            (true, Kind::AndAnd | Kind::Word("and")) | (false, Kind::OrOr | Kind::Word("or"))
        ) {
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
            Kind::NotEqual => crate::ComparisonOp::NotEqual,
            Kind::LessEqual => crate::ComparisonOp::LessEqual,
            Kind::GreaterEqual => crate::ComparisonOp::GreaterEqual,
            Kind::Punctuation('<') => crate::ComparisonOp::Less,
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
        // Keep precedence handling iterative: another parser frame on every
        // operand would reduce the supported nested-call/parenthesis budget.
        let mut values = vec![self.unary(depth)?];
        let mut operators = Vec::<(crate::BinaryOp, u8)>::new();
        loop {
            let (kind, precedence) = match self.peek()? {
                Kind::Punctuation('+') => (crate::BinaryOp::Add, 1),
                Kind::Punctuation('-') => (crate::BinaryOp::Subtract, 1),
                Kind::Punctuation('*') => (crate::BinaryOp::Multiply, 2),
                Kind::Punctuation('/') => (crate::BinaryOp::Divide, 2),
                Kind::Punctuation('%') => (crate::BinaryOp::Remainder, 2),
                Kind::Punctuation('&') => (crate::BinaryOp::BitAnd, 2),
                Kind::Punctuation('|') => (crate::BinaryOp::BitOr, 1),
                Kind::Punctuation('^') => (crate::BinaryOp::BitXor, 1),
                Kind::ShiftLeft => (crate::BinaryOp::ShiftLeft, 2),
                Kind::ShiftRight => (crate::BinaryOp::ShiftRight, 2),
                _ => break,
            };
            while operators
                .last()
                .is_some_and(|(_, previous)| *previous >= precedence)
            {
                let (previous, _) = operators.pop().unwrap();
                self.reduce_arithmetic(&mut values, previous)?;
            }
            self.token()?;
            operators.push((kind, precedence));
            values.push(self.unary(depth)?);
        }
        while let Some((kind, _)) = operators.pop() {
            self.reduce_arithmetic(&mut values, kind)?;
        }
        Ok(values.pop().unwrap())
    }

    /// Combine the last two parsed operands into a spanned binary expression.
    /// The operator stack only requests reduction after both operands exist.
    fn reduce_arithmetic(
        &mut self,
        values: &mut Vec<Expression>,
        kind: crate::BinaryOp,
    ) -> Result<(), Diagnostic> {
        let right = values.pop().unwrap();
        let left = values.pop().unwrap();
        let span = Span {
            start: left.span.start,
            end: right.span.end,
        };
        values.push(self.node(
            ExpressionKind::Binary {
                kind,
                left: Box::new(left),
                right: Box::new(right),
            },
            span,
        )?);
        Ok(())
    }
    /// Parse Number complement as a spanned unary expression.
    /// Isolate construction so ordinary unary parsing keeps its stack budget.
    fn complement(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        if depth >= 64 {
            return Err(self.error(self.offset, "expression nesting exceeds 64"));
        }
        let sign = self.token()?;
        let operand = self.unary(depth + 1)?;
        let span = Span {
            start: sign.span.start,
            end: operand.span.end,
        };
        self.node(ExpressionKind::Complement(Box::new(operand)), span)
    }
    /// Parse unary plus with its operand span and the ordinary nesting limit.
    /// Keep construction outside unary parsing to preserve its stack budget.
    fn positive(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let sign = self.token()?;
        if depth >= 64 {
            return Err(self.error(sign.span.start, "expression nesting exceeds 64 levels"));
        }
        let operand = self.primary(depth + 1)?;
        let span = Span {
            start: sign.span.start,
            end: operand.span.end,
        };
        self.node(ExpressionKind::Positive(Box::new(operand)), span)
    }
    fn unary(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        if self.peek()? == Kind::Punctuation('+') {
            return self.positive(depth);
        }
        if self.peek()? == Kind::Punctuation('~') {
            return self.complement(depth);
        }
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
    /// Parse Dictionary expressions while excluding nested Dictionary literal syntax.
    fn dictionary_literal(&mut self, start: usize, depth: usize) -> Result<Expression, Diagnostic> {
        if self.dictionary_entry {
            return Err(self.error(start, "nested Dictionary literal syntax is unsupported"));
        }
        if self.peek()? == Kind::Punctuation('}') {
            let close = self.token()?;
            return self.node(
                ExpressionKind::DictionaryNew,
                Span {
                    start,
                    end: close.span.end,
                },
            );
        }
        let mut entries = Vec::new();
        loop {
            if depth >= 64 {
                return Err(self.error(start, "expression nesting exceeds 64 levels"));
            }
            self.dictionary_entry = true;
            let key = self.expression(depth + 1)?;
            self.expect(
                Kind::Punctuation('='),
                "'=>' between Dictionary key and value",
            )?;
            self.expect(
                Kind::Punctuation('>'),
                "'=>' between Dictionary key and value",
            )?;
            let value = self.expression(depth + 1)?;
            self.dictionary_entry = false;
            entries.push((key, value));
            if self.peek()? != Kind::Punctuation(',') {
                break;
            }
            self.token()?;
        }
        let close = self.expect(Kind::Punctuation('}'), "'}'")?;
        self.node(
            ExpressionKind::DictionaryLiteral(entries),
            Span {
                start,
                end: close.span.end,
            },
        )
    }

    /// Parse construction outside ordinary primary-expression frames so nested
    /// calls retain the supported expression-depth budget on the default stack.
    fn construct(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let open = self.token()?;
        let path = self.path()?;
        let native = self
            .names
            .view(&path)
            .then_some(crate::NativeClass::View)
            .or_else(|| {
                self.names
                    .behavior_delegate(&path)
                    .then_some(crate::NativeClass::BehaviorDelegate)
            });
        if native.is_none() && path.contains('.') {
            return Err(self.error(open.span.start, "unsupported qualified construction class"));
        }
        let (arguments, end) = self.call_arguments(open.span.start, depth)?;
        if native.is_some() && !arguments.is_empty() {
            return Err(self.error(
                open.span.start,
                "supported native construction requires zero arguments",
            ));
        }
        self.node(
            if let Some(class) = native {
                ExpressionKind::NativeNew {
                    class,
                    qualifier: path.split('.').next().unwrap().into(),
                }
            } else {
                ExpressionKind::SourceNew {
                    class: path,
                    arguments,
                }
            },
            Span {
                start: open.span.start,
                end,
            },
        )
    }

    fn primary(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let mut expression = if self.peek()? == Kind::Punctuation('[') {
            let open = self.token()?;
            if depth >= 64 {
                return Err(self.error(open.span.start, "expression nesting exceeds 64 levels"));
            }
            let mut elements = if self.peek()? == Kind::Punctuation(']') {
                Vec::new()
            } else {
                vec![self.expression(depth + 1)?]
            };
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
        } else if self.peek()? == Kind::Word("new") {
            self.construct(depth)?
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
            Kind::Punctuation('{') => self.dictionary_literal(token.span.start, depth),
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
                    let member_end = member.span.end;
                    let Kind::Word(member) = member.kind else {
                        unreachable!()
                    };
                    if name == "self" && self.peek()? != Kind::Punctuation('(') {
                        return self.node(
                            ExpressionKind::SelfField(member.into()),
                            Span {
                                start: token.span.start,
                                end: member_end,
                            },
                        );
                    }
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
                    if self.names.module(module) == Some(crate::names::NativeModule::Graphics)
                        && member == "FONT_MEDIUM"
                        && self.peek()? != Kind::Punctuation('(')
                    {
                        return self.node(
                            ExpressionKind::NativeConstant {
                                module: "Toybox.Graphics".into(),
                                member: member.into(),
                                qualifier: path.split('.').next().unwrap().into(),
                            },
                            Span {
                                start: token.span.start,
                                end: self.offset,
                            },
                        );
                    }
                    let module = if let Some(native) = self.names.module(module) {
                        native.name()
                    } else if self.native_base.is_some()
                        && (module == "AppBase" || self.names.app_base(module))
                    {
                        "Application.AppBase"
                    } else if self.native_base.is_some()
                        && (module == "View" || self.names.view(module))
                    {
                        "WatchUi.View"
                    } else if self.native_base.is_some()
                        && (module == "BehaviorDelegate" || self.names.behavior_delegate(module))
                    {
                        "WatchUi.BehaviorDelegate"
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

    /// Check a parsed assignment target without entering RHS parsing.
    /// Local targets must be bare; explicit self fields and array elements are accepted.
    fn assignment_target(&self, expression: &Expression, start: usize) -> Result<(), Diagnostic> {
        if let ExpressionKind::Local(name) = &expression.kind {
            if self.source[expression.span.start..expression.span.end] != *name {
                return Err(self.error(start, "assignment target must be a bare local identifier"));
            }
        }
        if !matches!(
            expression.kind,
            ExpressionKind::Local(_)
                | ExpressionKind::SelfField(_)
                | ExpressionKind::ArrayRead { .. }
        ) {
            return Err(self.error(
                start,
                "assignment target must be a local, self field, or array element",
            ));
        }
        Ok(())
    }

    /// Construct the checked assignment after its RHS and terminator are parsed.
    /// Keep statement construction outside the expression parser's stack budget.
    fn assignment_statement(
        &mut self,
        expression: Expression,
        assignment: Kind<'_>,
        value: Expression,
        span: Span,
    ) -> Result<Statement, Diagnostic> {
        let statement = if let Kind::Compound(kind) = assignment {
            self.compound_statement(expression, kind, value, span)?
        } else {
            match expression.kind {
                ExpressionKind::ArrayRead { array, index } => Statement::ArrayAssign {
                    array: *array,
                    index: *index,
                    value,
                    span,
                },
                ExpressionKind::SelfField(name) => Statement::FieldAssign { name, value, span },
                ExpressionKind::Local(name) => Statement::Assign { name, value, span },
                _ => unreachable!("checked assignment target"),
            }
        };
        Ok(statement)
    }

    /// Construct a compound local/field read and write, or defer an indexed target.
    /// Source lowering captures array locations and checks operand/result contracts.
    fn compound_statement(
        &mut self,
        target: Expression,
        kind: crate::BinaryOp,
        value: Expression,
        span: Span,
    ) -> Result<Statement, Diagnostic> {
        let (name, field) = match &target.kind {
            ExpressionKind::ArrayRead { .. } => {
                return Ok(Statement::ArrayCompound {
                    target,
                    kind,
                    value,
                    span,
                });
            }
            ExpressionKind::Local(name) => (name.clone(), false),
            ExpressionKind::SelfField(name) => (name.clone(), true),
            _ => unreachable!("checked compound target"),
        };
        let value_span = Span {
            start: target.span.start,
            end: value.span.end,
        };
        let value = self.node(
            ExpressionKind::Binary {
                kind,
                left: Box::new(target),
                right: Box::new(value),
            },
            value_span,
        )?;
        Ok(if field {
            Statement::FieldAssign { name, value, span }
        } else {
            Statement::Assign { name, value, span }
        })
    }

    /// Parse a prefix statement update outside the recursive block parser frame.
    fn prefix_update(&mut self, start: usize, increment: bool) -> Result<Statement, Diagnostic> {
        self.token()?;
        let target = self.expression(0)?;
        self.finish_update(target, increment, start)
    }

    /// Check an update target and require its statement terminator.
    fn finish_update(
        &mut self,
        target: Expression,
        increment: bool,
        start: usize,
    ) -> Result<Statement, Diagnostic> {
        self.assignment_target(&target, start)?;
        let end = self
            .expect(Kind::Punctuation(';'), "';' (updates are statements)")?
            .span
            .end;
        Ok(Statement::Update {
            target,
            increment,
            span: Span { start, end },
        })
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
                Kind::Step(increment) => self.prefix_update(start, increment)?,
                Kind::End => return Err(self.error(start, "expected '}' before end of input")),
                _ => {
                    let expression = self.expression(0)?;
                    if let Kind::Step(increment) = self.peek()? {
                        self.token()?;
                        self.finish_update(expression, increment, start)?
                    } else if matches!(self.peek()?, Kind::Punctuation('=') | Kind::Compound(_)) {
                        let assignment = self.token()?.kind;
                        self.assignment_target(&expression, start)?;
                        let value = self.expression(0)?;
                        let end = self.expect(Kind::Punctuation(';'), "';'")?.span.end;
                        self.assignment_statement(
                            expression,
                            assignment,
                            value,
                            Span { start, end },
                        )?
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
                            if matches!(
                                module.as_str(),
                                "Application.AppBase" | "WatchUi.View" | "WatchUi.BehaviorDelegate"
                            ) {
                                if member != "initialize" || !arguments.is_empty() {
                                    return Err(self.error(
                                        start,
                                        "only a zero-argument native base initialize call is supported",
                                    ));
                                }
                                Statement::BaseInitialize {
                                    span: Span { start, end },
                                    base: if module == "Application.AppBase" {
                                        crate::ClassBase::ApplicationAppBase
                                    } else if module == "WatchUi.View" {
                                        crate::ClassBase::WatchUiView
                                    } else {
                                        crate::ClassBase::WatchUiBehaviorDelegate
                                    },
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
                            } else if module == "WatchUi" {
                                Statement::NativeModuleCall {
                                    module: "Toybox.WatchUi".into(),
                                    member,
                                    arguments,
                                    span: Span { start, end },
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
        dictionary_entry: false,
        names: crate::names::Names::default(),
        native_base: None,
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
                        "Toybox.Lang.Dictionary" => self
                            .dictionary_annotation()?
                            .map(crate::ParameterType::TypedDictionary)
                            .unwrap_or(crate::ParameterType::Dictionary),
                        "Toybox.WatchUi.View" => {
                            crate::ParameterType::Object(crate::NativeClass::View)
                        }
                        "Toybox.WatchUi.BehaviorDelegate" => {
                            crate::ParameterType::Object(crate::NativeClass::BehaviorDelegate)
                        }
                        "Toybox.Graphics.Dc" => {
                            crate::ParameterType::Object(crate::NativeClass::Dc)
                        }
                        "Toybox.Lang.Array" => {
                            crate::ParameterType::Array(self.array_annotation()?)
                        }
                        name if !name.contains('.')
                            && !matches!(
                                name,
                                "Number"
                                    | "String"
                                    | "Boolean"
                                    | "Char"
                                    | "Array"
                                    | "Object"
                                    | "View"
                                    | "Dc"
                                    | "Void"
                            ) =>
                        {
                            crate::ParameterType::SourceObject(name.into())
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
                "Toybox.Lang.Dictionary" => self
                    .dictionary_annotation()?
                    .map(crate::ReturnType::TypedDictionary)
                    .unwrap_or(crate::ReturnType::Dictionary),
                "Toybox.WatchUi.View" => crate::ReturnType::Object(crate::NativeClass::View),
                "Toybox.WatchUi.BehaviorDelegate" => {
                    crate::ReturnType::Object(crate::NativeClass::BehaviorDelegate)
                }
                "Toybox.Lang.Array" => crate::ReturnType::Array(self.array_annotation()?),
                "Void" => crate::ReturnType::Void,
                name if !name.contains('.')
                    && !matches!(
                        name,
                        "Number"
                            | "String"
                            | "Boolean"
                            | "Char"
                            | "Array"
                            | "Object"
                            | "View"
                            | "Dc"
                    ) =>
                {
                    crate::ReturnType::SourceObject(name.into())
                }
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
    pub base: crate::ClassBase,
    pub fields: Vec<crate::InstanceField>,
    pub methods: Vec<ParsedFunction>,
}

/// Parse global/module functions and native-derived class methods/Number fields.
/// Collects source ownership without binding calls or selecting VM instructions.
pub(crate) fn declarations(source: &str) -> Result<ParsedUnit, Diagnostic> {
    let mut parser = Parser {
        source,
        offset: 0,
        lookahead: None,
        expressions: 0,
        dictionary_entry: false,
        names: crate::names::Names::default(),
        native_base: None,
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
            let mut class_base = None;
            if is_class {
                parser.expect(Kind::Word("extends"), "'extends Application.AppBase'")?;
                let base = parser.path()?;
                class_base = if parser.names.app_base(&base) {
                    Some(crate::ClassBase::ApplicationAppBase)
                } else if parser.names.view(&base) {
                    Some(crate::ClassBase::WatchUiView)
                } else if parser.names.behavior_delegate(&base) {
                    Some(crate::ClassBase::WatchUiBehaviorDelegate)
                } else {
                    None
                };
                if class_base.is_none() {
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
            parser.native_base = class_base;
            parser.imports()?;
            let mut methods = Vec::new();
            let mut fields = Vec::new();
            while parser.peek()? != Kind::Punctuation('}') {
                if is_class && parser.peek()? == Kind::Word("var") {
                    parser.token()?;
                    let token = parser.identifier()?;
                    parser.reject_binding(&token)?;
                    let Kind::Word(name) = token.kind else {
                        unreachable!()
                    };
                    let kind = if parser.peek()? == Kind::Word("as") {
                        parser.token()?;
                        let class = parser.path()?;
                        if class.contains('.') {
                            return Err(parser.error(
                                token.span.start,
                                "reference field requires a source View class name",
                            ));
                        }
                        parser.expect(
                            Kind::Punctuation(';'),
                            "';' after reference field declaration",
                        )?;
                        crate::FieldKind::SourceObject(class)
                    } else {
                        parser.expect(Kind::Punctuation('='), "'=' and a Number literal")?;
                        let value = parser.expression(0)?;
                        let ExpressionKind::Integer(initial) = value.kind else {
                            return Err(parser.error(
                                value.span.start,
                                "instance fields require a Number literal initializer",
                            ));
                        };
                        parser.expect(Kind::Punctuation(';'), "';'")?;
                        crate::FieldKind::Number(initial)
                    };
                    fields.push(crate::InstanceField {
                        name: name.into(),
                        kind,
                    });
                    if fields.len() > 256 {
                        return Err(
                            parser.error(token.span.start, "class exceeds 256 instance fields")
                        );
                    }
                    continue;
                }
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
            parser.native_base = None;
            if methods.is_empty() {
                return Err(parser.error(token.span.start, "empty modules/classes are unsupported"));
            }
            if is_class {
                unit.classes.push(ParsedClass {
                    name: name.into(),
                    base: class_base.unwrap(),
                    fields,
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
