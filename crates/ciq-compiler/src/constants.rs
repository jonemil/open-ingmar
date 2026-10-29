//! Immediate source constants, resolved before semantic IR construction.
use crate::{
    Span,
    parser::{Expression, ExpressionKind as E},
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Immediate {
    Number(i32),
    Float(u32),
    Boolean(bool),
    Char(u32),
    Symbol(String),
    Null,
}
impl Immediate {
    pub(crate) fn api_type(&self) -> crate::api::Type {
        use crate::api::Type;
        Type::named(match self {
            Self::Number(_) => "Toybox.Lang.Number",
            Self::Float(_) => "Toybox.Lang.Float",
            Self::Boolean(_) => "Toybox.Lang.Boolean",
            Self::Char(_) => "Toybox.Lang.Char",
            Self::Symbol(_) => "Toybox.Lang.Symbol",
            Self::Null => return Type::Null,
        })
    }
    pub(crate) fn expression(&self, span: Span) -> Expression {
        let kind = match self {
            Self::Number(v) => E::Integer(*v),
            Self::Float(v) => E::Float(*v),
            Self::Boolean(v) => E::Boolean(*v),
            Self::Char(v) => E::Char(*v),
            Self::Symbol(v) => E::Symbol(v.clone()),
            Self::Null => E::Null,
        };
        Expression { kind, span }
    }
}
/// Recognize a declaration-qualified name without treating arbitrary receivers
/// or calls as namespace lookup. Lexical shadowing is checked by the caller.
pub(crate) fn path(expression: &Expression) -> Option<String> {
    let mut expression = expression;
    let mut members = Vec::new();
    while let E::MemberRead { receiver, member } = &expression.kind {
        members.push(member.as_str());
        expression = receiver;
    }
    let mut path = match &expression.kind {
        E::Local(name) => name.clone(),
        E::SelfField(name) => format!("self.{name}"),
        _ => return None,
    };
    for member in members.into_iter().rev() {
        path.push('.');
        path.push_str(member);
    }
    Some(path)
}

/// Read a non-heap scalar literal for direct substitution in source bodies.
/// References and operators require ordered initialization even when their
/// values could be computed in advance; folding them would change Null reads.
pub(crate) fn literal(expression: &Expression) -> Option<Immediate> {
    Some(match &expression.kind {
        E::Integer(v) => Immediate::Number(*v),
        E::Float(v) => Immediate::Float(*v),
        E::Boolean(v) => Immediate::Boolean(*v),
        E::Char(v) => Immediate::Char(*v),
        E::Symbol(v) => Immediate::Symbol(v.clone()),
        E::Null => Immediate::Null,
        _ => return None,
    })
}

/// Classify ordinary constant defaults before source forms lose grouping.
/// Class aggregate defaults remain runtime initializers under the SDK profile.
pub(crate) fn literal_default(expression: &Expression, class_field: bool, source: &str) -> bool {
    if source.as_bytes().get(expression.span.start) == Some(&b'(') {
        return false;
    }
    fn scalar(expression: &Expression) -> bool {
        matches!(
            expression.kind,
            E::String(_)
                | E::Long(_)
                | E::Double(_)
                | E::Integer(_)
                | E::Float(_)
                | E::Boolean(_)
                | E::Char(_)
                | E::Symbol(_)
                | E::Null
        )
    }
    scalar(expression) || matches!(&expression.kind, E::Positive(inner)
        if matches!(inner.kind, E::Integer(_))
            && source.as_bytes().get(inner.span.start).is_some_and(u8::is_ascii_digit))
    || (!class_field && match &expression.kind {
        E::ArrayLiteral(elements) | E::ByteArrayLiteral(elements) => {
            !elements.is_empty() && elements.iter().all(|element| {
                scalar(element) || matches!(&element.kind, E::Positive(value)
                    if matches!(value.kind, E::Integer(_) | E::Float(_) | E::Long(_) | E::Double(_)))
            })
        }
        _ => false,
    })
}
