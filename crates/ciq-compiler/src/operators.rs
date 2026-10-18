//! Operator contracts for the executable source subset, before VM selection.
//! These rules do not enumerate every operator/type combination in Monkey C.
use crate::{BinaryOp, ComparisonOp, api::Type};

/// Resolve Boolean inversion or integer complement without changing its type.
pub(crate) fn not_result(operand: &Type) -> Option<Type> {
    (*operand == Type::named("Toybox.Lang.Boolean") || integer(operand)).then(|| operand.clone())
}

/// Resolve Number/Long complement without widening its width or admitting Null.
pub(crate) fn complement_result(operand: &Type) -> Option<Type> {
    integer(operand).then(|| operand.clone())
}

/// Resolve unary signs without changing numeric width or admitting Null.
pub(crate) fn sign_result(operand: &Type) -> Option<Type> {
    numeric(operand).then(|| operand.clone())
}

fn integer(ty: &Type) -> bool {
    matches!(ty, Type::Named(name) if matches!(name.as_str(), "Toybox.Lang.Number" | "Toybox.Lang.Long"))
}

fn numeric(ty: &Type) -> bool {
    integer(ty)
        || matches!(ty, Type::Named(name) if matches!(name.as_str(), "Toybox.Lang.Float" | "Toybox.Lang.Double"))
}

/// Resolve a numeric step or Char increment, preserving the operand type.
pub(crate) fn step_result(operand: &Type, increment: bool) -> Option<Type> {
    (numeric(operand) || (increment && *operand == Type::named("Toybox.Lang.Char")))
        .then(|| operand.clone())
}

/// Resolve supported arithmetic operand types to their semantic result type.
/// Number/Long arithmetic, bitwise operations, and shifts promote to Long when
/// either operand is Long. Addition of Char/integer in either
/// order returns Char; two Chars concatenate to String. Floating arithmetic
/// promotes to Double for Double operands or a Long/Float pair, otherwise Float.
/// An exact String operand
/// concatenates supported scalar/null alternatives. Other pairs are rejected.
pub(crate) fn binary_result(kind: BinaryOp, left: &Type, right: &Type) -> Option<Type> {
    if matches!(
        kind,
        BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide
    ) && numeric(left)
        && numeric(right)
    {
        let double = Type::named("Toybox.Lang.Double");
        let float = Type::named("Toybox.Lang.Float");
        let long = Type::named("Toybox.Lang.Long");
        if left == &double
            || right == &double
            || ((left == &float && right == &long) || (left == &long && right == &float))
        {
            return Some(double);
        }
        if left == &float || right == &float {
            return Some(float);
        }
    }
    match kind {
        BinaryOp::Add
            if (*left == Type::named("Toybox.Lang.String") && string_operand(right, true))
                || (*right == Type::named("Toybox.Lang.String") && string_operand(left, true)) =>
        {
            Some(Type::named("Toybox.Lang.String"))
        }
        BinaryOp::Add
        | BinaryOp::Subtract
        | BinaryOp::Multiply
        | BinaryOp::Divide
        | BinaryOp::Remainder
        | BinaryOp::BitAnd
        | BinaryOp::BitOr
        | BinaryOp::BitXor
        | BinaryOp::ShiftLeft
        | BinaryOp::ShiftRight
            if integer(left)
                && integer(right)
                && (*left == Type::named("Toybox.Lang.Long")
                    || *right == Type::named("Toybox.Lang.Long")) =>
        {
            Some(Type::named("Toybox.Lang.Long"))
        }
        BinaryOp::Add
        | BinaryOp::Subtract
        | BinaryOp::Multiply
        | BinaryOp::Divide
        | BinaryOp::Remainder
        | BinaryOp::BitAnd
        | BinaryOp::BitOr
        | BinaryOp::BitXor
        | BinaryOp::ShiftLeft
        | BinaryOp::ShiftRight
            if *left == Type::named("Toybox.Lang.Number") && left == right =>
        {
            Some(Type::named("Toybox.Lang.Number"))
        }
        BinaryOp::Add if *left == Type::named("Toybox.Lang.Char") && left == right => {
            Some(Type::named("Toybox.Lang.String"))
        }
        BinaryOp::Add
            if (*left == Type::named("Toybox.Lang.Char") && integer(right))
                || (integer(left) && *right == Type::named("Toybox.Lang.Char")) =>
        {
            Some(Type::named("Toybox.Lang.Char"))
        }
        _ => None,
    }
}

/// Resolve supported comparison types to Boolean. Number/String/Dictionary-or-null values
/// support equality/inequality with Null; Boolean pairs support these two operators.
/// Numeric scalar pairs and Number/Char pairs support all six comparisons.
/// Char pairs with wider numeric types support equality only.
/// String equality uses object identity with supported String/numeric/Char/null
/// alternatives; Boolean mixing and String ordering remain unsupported.
pub(crate) fn comparison_result(kind: ComparisonOp, left: &Type, right: &Type) -> Option<Type> {
    let char_numeric = |ty: &Type| matches!(ty, Type::Named(name) if matches!(name.as_str(), "Toybox.Lang.Number" | "Toybox.Lang.Char"));
    let char_equality = kind.is_equality()
        && ((*left == Type::named("Toybox.Lang.Char") && numeric(right))
            || (numeric(left) && *right == Type::named("Toybox.Lang.Char")));
    let numbers = (char_numeric(left) && char_numeric(right))
        || (numeric(left) && numeric(right))
        || char_equality;
    let symbols = kind.is_equality() && *left == Type::named("Toybox.Lang.Symbol") && left == right;
    let booleans =
        kind.is_equality() && *left == Type::named("Toybox.Lang.Boolean") && left == right;
    let nullable_object = Type::Union(vec![Type::named("Toybox.Lang.Object"), Type::Null]);
    let nullable_dictionary = Type::Union(vec![Type::named("Toybox.Lang.Dictionary"), Type::Null]);
    let nullable_number = Type::Union(vec![Type::named("Toybox.Lang.Number"), Type::Null]);
    let nullable_string = Type::Union(vec![Type::named("Toybox.Lang.String"), Type::Null]);
    let nullable_boolean = Type::Union(vec![Type::named("Toybox.Lang.Boolean"), Type::Null]);
    let nullable_char = Type::Union(vec![Type::named("Toybox.Lang.Char"), Type::Null]);
    let nullable_symbol = Type::Union(vec![Type::named("Toybox.Lang.Symbol"), Type::Null]);
    let nullable_float = Type::Union(vec![Type::named("Toybox.Lang.Float"), Type::Null]);
    let nullable_double = Type::Union(vec![Type::named("Toybox.Lang.Double"), Type::Null]);
    let nullable_long = Type::Union(vec![Type::named("Toybox.Lang.Long"), Type::Null]);
    let interface = |ty: &Type| {
        matches!(
            crate::ValueType::from_api_type(ty),
            Some(crate::ValueType::Interface(_) | crate::ValueType::NullableInterface(_))
        )
    };
    let interface_null = kind.is_equality()
        && ((*left == Type::Null && interface(right)) || (interface(left) && *right == Type::Null));
    let null_check = kind.is_equality()
        && ((left == &Type::Null
            && (right == &Type::Null
                || right == &nullable_object
                || right == &nullable_dictionary
                || right == &nullable_number
                || right == &nullable_string
                || right == &nullable_boolean
                || right == &nullable_symbol
                || right == &nullable_char
                || right == &nullable_float
                || right == &nullable_double
                || right == &nullable_long))
            || ((left == &nullable_object
                || left == &nullable_dictionary
                || left == &nullable_number
                || left == &nullable_string
                || left == &nullable_boolean
                || left == &nullable_symbol
                || left == &nullable_char
                || left == &nullable_float
                || left == &nullable_double
                || left == &nullable_long)
                && right == &Type::Null));
    let strings = kind.is_equality()
        && ((*left == Type::named("Toybox.Lang.String") && string_operand(right, false))
            || (*right == Type::named("Toybox.Lang.String") && string_operand(left, false)));
    (numbers || symbols || booleans || null_check || interface_null || strings)
        .then(|| Type::named("Toybox.Lang.Boolean"))
}

fn string_operand(ty: &Type, boolean: bool) -> bool {
    match ty {
        Type::Null => true,
        Type::Named(name) => {
            matches!(
                name.as_str(),
                "Toybox.Lang.String"
                    | "Toybox.Lang.Number"
                    | "Toybox.Lang.Long"
                    | "Toybox.Lang.Float"
                    | "Toybox.Lang.Double"
                    | "Toybox.Lang.Char"
            ) || (boolean && name == "Toybox.Lang.Boolean")
        }
        Type::Union(types) => types.iter().all(|ty| string_operand(ty, boolean)),
        Type::Void
        | Type::Array(_)
        | Type::Dictionary { .. }
        | Type::Tuple(_)
        | Type::Parameter(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ValueType;

    #[test]
    fn operator_rules_preserve_supported_types_and_reject_other_pairs() {
        let types = [
            ValueType::Integer,
            ValueType::Long,
            ValueType::NullableLong,
            ValueType::Float,
            ValueType::NullableFloat,
            ValueType::Double,
            ValueType::NullableDouble,
            ValueType::NullableInteger,
            ValueType::String,
            ValueType::NullableString,
            ValueType::Boolean,
            ValueType::NullableBoolean,
            ValueType::NullableChar,
            ValueType::Char,
            ValueType::Array(crate::ArrayElement::Char),
            ValueType::Array(crate::ArrayElement::Number),
            ValueType::Null,
            ValueType::NullableDictionary,
        ];
        let numeric_types = [
            ValueType::Integer,
            ValueType::Long,
            ValueType::Float,
            ValueType::Double,
        ];
        use ValueType::{Double as D, Float as F, Integer as N, Long as L};
        let promotion = [[N, L, F, D], [L, L, D, D], [F, D, F, D], [D, D, D, D]];
        for left in types {
            for right in types {
                let lhs = left.api_type();
                let rhs = right.api_type();
                for op in [
                    BinaryOp::Add,
                    BinaryOp::Subtract,
                    BinaryOp::Multiply,
                    BinaryOp::Divide,
                    BinaryOp::Remainder,
                    BinaryOp::BitAnd,
                    BinaryOp::BitOr,
                    BinaryOp::BitXor,
                    BinaryOp::ShiftLeft,
                    BinaryOp::ShiftRight,
                ] {
                    let result = binary_result(op, &lhs, &rhs);
                    let expected = match (op, left, right) {
                        (BinaryOp::Add, ValueType::String, other)
                        | (BinaryOp::Add, other, ValueType::String)
                            if matches!(
                                other,
                                ValueType::Integer
                                    | ValueType::Long
                                    | ValueType::Float
                                    | ValueType::Double
                                    | ValueType::NullableLong
                                    | ValueType::NullableFloat
                                    | ValueType::NullableDouble
                                    | ValueType::NullableInteger
                                    | ValueType::String
                                    | ValueType::NullableString
                                    | ValueType::NullableChar
                                    | ValueType::Boolean
                                    | ValueType::NullableBoolean
                                    | ValueType::Char
                                    | ValueType::Null
                            ) =>
                        {
                            Some(ValueType::String)
                        }
                        (
                            BinaryOp::Add
                            | BinaryOp::Subtract
                            | BinaryOp::Multiply
                            | BinaryOp::Divide,
                            left,
                            right,
                        ) if numeric_types.contains(&left) && numeric_types.contains(&right) => {
                            let l = numeric_types.iter().position(|ty| *ty == left).unwrap();
                            let r = numeric_types.iter().position(|ty| *ty == right).unwrap();
                            Some(promotion[l][r])
                        }
                        (_, left, right)
                            if matches!(left, ValueType::Integer | ValueType::Long)
                                && matches!(right, ValueType::Integer | ValueType::Long)
                                && (left == ValueType::Long || right == ValueType::Long) =>
                        {
                            Some(ValueType::Long)
                        }
                        (_, ValueType::Integer, ValueType::Integer) => Some(ValueType::Integer),
                        (BinaryOp::Add, ValueType::Char, ValueType::Char) => {
                            Some(ValueType::String)
                        }
                        (BinaryOp::Add, ValueType::Integer | ValueType::Long, ValueType::Char)
                        | (BinaryOp::Add, ValueType::Char, ValueType::Integer | ValueType::Long) => {
                            Some(ValueType::Char)
                        }
                        _ => None,
                    };
                    assert_eq!(
                        result,
                        expected.map(ValueType::api_type),
                        "{op:?}: {left:?}, {right:?}"
                    );
                }
                for op in [
                    ComparisonOp::Equal,
                    ComparisonOp::Greater,
                    ComparisonOp::Less,
                    ComparisonOp::LessEqual,
                    ComparisonOp::GreaterEqual,
                    ComparisonOp::NotEqual,
                ] {
                    let result = comparison_result(op, &lhs, &rhs);
                    let null_check = op.is_equality()
                        && matches!(
                            (left, right),
                            (
                                ValueType::Null,
                                ValueType::Null
                                    | ValueType::NullableDictionary
                                    | ValueType::NullableInteger
                                    | ValueType::NullableString
                                    | ValueType::NullableBoolean
                                    | ValueType::NullableChar
                                    | ValueType::NullableFloat
                                    | ValueType::NullableDouble
                                    | ValueType::NullableLong
                            ) | (
                                ValueType::NullableDictionary
                                    | ValueType::NullableInteger
                                    | ValueType::NullableString
                                    | ValueType::NullableBoolean
                                    | ValueType::NullableChar
                                    | ValueType::NullableFloat
                                    | ValueType::NullableDouble
                                    | ValueType::NullableLong,
                                ValueType::Null
                            )
                        );
                    let strings = op.is_equality()
                        && matches!(
                            (left, right),
                            (
                                ValueType::String,
                                ValueType::Integer
                                    | ValueType::Long
                                    | ValueType::Float
                                    | ValueType::Double
                                    | ValueType::NullableLong
                                    | ValueType::NullableFloat
                                    | ValueType::NullableDouble
                                    | ValueType::NullableInteger
                                    | ValueType::String
                                    | ValueType::NullableString
                                    | ValueType::NullableChar
                                    | ValueType::Char
                                    | ValueType::Null
                            ) | (
                                ValueType::Integer
                                    | ValueType::Long
                                    | ValueType::Float
                                    | ValueType::Double
                                    | ValueType::NullableLong
                                    | ValueType::NullableFloat
                                    | ValueType::NullableDouble
                                    | ValueType::NullableInteger
                                    | ValueType::NullableString
                                    | ValueType::NullableChar
                                    | ValueType::Char
                                    | ValueType::Null,
                                ValueType::String
                            )
                        );
                    assert_eq!(
                        result.is_some(),
                        (matches!(left, ValueType::Integer | ValueType::Char)
                            && matches!(right, ValueType::Integer | ValueType::Char))
                            || (numeric_types.contains(&left) && numeric_types.contains(&right))
                            || (op.is_equality()
                                && ((left == ValueType::Char && numeric_types.contains(&right))
                                    || (right == ValueType::Char
                                        && numeric_types.contains(&left))))
                            || null_check
                            || strings
                            || (op.is_equality()
                                && left == ValueType::Boolean
                                && right == ValueType::Boolean),
                        "{op:?}: {left:?}, {right:?}"
                    );
                    if let Some(result) = result {
                        assert_eq!(result, ValueType::Boolean.api_type());
                    }
                }
            }
        }
        assert!(binary_result(BinaryOp::Add, &Type::Void, &Type::Void).is_none());
        assert!(
            comparison_result(ComparisonOp::Equal, &Type::named("Unknown"), &Type::Null).is_none()
        );
    }

    #[test]
    fn subset_value_types_round_trip_through_known_api_identities() {
        for ty in [
            ValueType::Integer,
            ValueType::Long,
            ValueType::NullableLong,
            ValueType::Float,
            ValueType::NullableFloat,
            ValueType::Double,
            ValueType::NullableDouble,
            ValueType::NullableInteger,
            ValueType::String,
            ValueType::NullableString,
            ValueType::Boolean,
            ValueType::NullableBoolean,
            ValueType::NullableChar,
            ValueType::Char,
            ValueType::Array(crate::ArrayElement::Char),
            ValueType::Array(crate::ArrayElement::Number),
            ValueType::Null,
            ValueType::NullableDictionary,
        ] {
            let api_type = ty.api_type();
            assert_eq!(ValueType::from_api_type(&api_type), Some(ty));
            let names = match api_type {
                Type::Named(name) => vec![name],
                Type::Union(types) => types
                    .into_iter()
                    .filter_map(|ty| match ty {
                        Type::Named(name) => Some(name),
                        _ => None,
                    })
                    .collect(),
                _ => vec![],
            };
            for name in names {
                assert!(crate::api::core().class(&name).is_some());
            }
        }
        assert_eq!(ValueType::from_api_type(&Type::Void), None);
        assert_eq!(
            ValueType::from_api_type(&Type::named("Toybox.Lang.Object")),
            Some(ValueType::LangObject)
        );
    }
}
