//! Immediate source constants, resolved before semantic IR construction.
use crate::{
    BinaryOp, Span,
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

/// Evaluate non-heap literals and the supported pure operators. Heap-backed
/// values and calls require declaration storage to preserve identity/lifetime
/// and initialization effects; they must not be reconstructed on each read.
pub(crate) fn evaluate(
    expression: &Expression,
    resolve: &mut impl FnMut(&str, usize) -> Result<Immediate, String>,
) -> Result<Immediate, String> {
    if let Some(path) = path(expression) {
        return resolve(&path, expression.span.start);
    }
    match &expression.kind {
        E::Integer(v) => Ok(Immediate::Number(*v)),
        E::Float(v) => Ok(Immediate::Float(*v)),
        E::Boolean(v) => Ok(Immediate::Boolean(*v)),
        E::Char(v) => Ok(Immediate::Char(*v)),
        E::Symbol(v) => Ok(Immediate::Symbol(v.clone())),
        E::Null => Ok(Immediate::Null),
        E::Positive(e) => match evaluate(e, resolve)? {
            v @ (Immediate::Number(_) | Immediate::Float(_)) => Ok(v),
            _ => Err("constant unary plus requires Number or Float".into()),
        },
        E::Negate(e) => match evaluate(e, resolve)? {
            Immediate::Number(v) => Ok(Immediate::Number(v.wrapping_neg())),
            Immediate::Float(v) => Ok(Immediate::Float(v ^ (1 << 31))),
            _ => Err("constant negation requires Number or Float".into()),
        },
        E::Not(e) => match evaluate(e, resolve)? {
            Immediate::Boolean(v) => Ok(Immediate::Boolean(!v)),
            _ => Err("constant logical negation requires Boolean".into()),
        },
        E::Complement(e) => match evaluate(e, resolve)? {
            Immediate::Number(v) => Ok(Immediate::Number(!v)),
            _ => Err("constant complement requires Number".into()),
        },
        E::Binary { kind, left, right } => {
            let (Immediate::Number(a), Immediate::Number(b)) =
                (evaluate(left, resolve)?, evaluate(right, resolve)?)
            else {
                return Err("constant binary expressions currently require Numbers".into());
            };
            let value = match kind {
                BinaryOp::Add => a.wrapping_add(b),
                BinaryOp::Subtract => a.wrapping_sub(b),
                BinaryOp::Multiply => a.wrapping_mul(b),
                BinaryOp::Divide if b != 0 => a.wrapping_div(b),
                BinaryOp::Remainder if b != 0 => a.wrapping_rem(b),
                BinaryOp::BitAnd => a & b,
                BinaryOp::BitOr => a | b,
                BinaryOp::BitXor => a ^ b,
                BinaryOp::ShiftLeft => a.wrapping_shl(b as u32),
                BinaryOp::ShiftRight => a.wrapping_shr(b as u32),
                _ => return Err("invalid or zero-divisor Number constant operation".into()),
            };
            Ok(Immediate::Number(value))
        }
        _ => Err(concat!(
            "constant initializer requires a supported immediate expression; ",
            "heap-backed values and runtime initializers need declaration storage"
        )
        .into()),
    }
}

/// Identify expressions needing shared storage rather than immediate expansion.
/// Pure unsupported scalar forms remain diagnostics until their profile is added.
pub(crate) fn storage_expression(expression: &Expression) -> bool {
    match &expression.kind {
        E::Integer(_)
        | E::Float(_)
        | E::Boolean(_)
        | E::Char(_)
        | E::Symbol(_)
        | E::Null
        | E::NativeConstant { .. } => false,
        E::Positive(e) | E::Negate(e) | E::Not(e) | E::Complement(e) => storage_expression(e),
        E::Binary { left, right, .. }
        | E::Compare { left, right, .. }
        | E::Logical { left, right, .. } => storage_expression(left) || storage_expression(right),
        E::Conditional {
            condition,
            when_true,
            when_false,
        } => {
            storage_expression(condition)
                || storage_expression(when_true)
                || storage_expression(when_false)
        }
        _ => true,
    }
}
