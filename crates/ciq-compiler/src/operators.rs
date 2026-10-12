//! Operator contracts for the executable source subset, before VM selection.
//! These rules do not enumerate every operator/type combination in Monkey C.
use crate::{BinaryOp, ComparisonOp, api::Type};

/// Resolve Number negation to Number without widening other or nullable types.
pub(crate) fn negate_result(operand: &Type) -> Option<Type> {
    (*operand == Type::named("Toybox.Lang.Number")).then(|| operand.clone())
}

/// Resolve supported arithmetic operand types to their semantic result type.
/// Number pairs support addition/subtraction. Addition of Char/Number in either
/// order returns Char; two Chars concatenate to String. An exact String operand
/// concatenates supported scalar/null alternatives. Other pairs are rejected.
pub(crate) fn binary_result(kind: BinaryOp, left: &Type, right: &Type) -> Option<Type> {
    match kind {
        BinaryOp::Add
            if (*left == Type::named("Toybox.Lang.String") && string_operand(right, true))
                || (*right == Type::named("Toybox.Lang.String") && string_operand(left, true)) =>
        {
            Some(Type::named("Toybox.Lang.String"))
        }
        BinaryOp::Add | BinaryOp::Subtract
            if *left == Type::named("Toybox.Lang.Number") && left == right =>
        {
            Some(Type::named("Toybox.Lang.Number"))
        }
        BinaryOp::Add if *left == Type::named("Toybox.Lang.Char") && left == right => {
            Some(Type::named("Toybox.Lang.String"))
        }
        BinaryOp::Add
            if (*left == Type::named("Toybox.Lang.Char")
                && *right == Type::named("Toybox.Lang.Number"))
                || (*left == Type::named("Toybox.Lang.Number")
                    && *right == Type::named("Toybox.Lang.Char")) =>
        {
            Some(Type::named("Toybox.Lang.Char"))
        }
        _ => None,
    }
}

/// Resolve supported comparison types to Boolean. Number/String/Dictionary-or-null values
/// support equality with Null; Boolean pairs support equality only.
/// Number/Char pairs support equality and greater comparison through numeric values.
/// String equality uses object identity with supported String/Number/Char/null
/// alternatives; Boolean mixing and String ordering remain unsupported.
pub(crate) fn comparison_result(kind: ComparisonOp, left: &Type, right: &Type) -> Option<Type> {
    let numeric = |ty: &Type| matches!(ty, Type::Named(name) if matches!(name.as_str(), "Toybox.Lang.Number" | "Toybox.Lang.Char"));
    let numbers = numeric(left) && numeric(right);
    let booleans =
        kind == ComparisonOp::Equal && *left == Type::named("Toybox.Lang.Boolean") && left == right;
    let nullable_dictionary = Type::Union(vec![Type::named("Toybox.Lang.Dictionary"), Type::Null]);
    let nullable_number = Type::Union(vec![Type::named("Toybox.Lang.Number"), Type::Null]);
    let nullable_string = Type::Union(vec![Type::named("Toybox.Lang.String"), Type::Null]);
    let null_check = kind == ComparisonOp::Equal
        && ((left == &Type::Null
            && (right == &Type::Null
                || right == &nullable_dictionary
                || right == &nullable_number
                || right == &nullable_string))
            || ((left == &nullable_dictionary
                || left == &nullable_number
                || left == &nullable_string)
                && right == &Type::Null));
    let strings = kind == ComparisonOp::Equal
        && ((*left == Type::named("Toybox.Lang.String") && string_operand(right, false))
            || (*right == Type::named("Toybox.Lang.String") && string_operand(left, false)));
    (numbers || booleans || null_check || strings).then(|| Type::named("Toybox.Lang.Boolean"))
}

fn string_operand(ty: &Type, boolean: bool) -> bool {
    match ty {
        Type::Null => true,
        Type::Named(name) => {
            matches!(
                name.as_str(),
                "Toybox.Lang.String" | "Toybox.Lang.Number" | "Toybox.Lang.Char"
            ) || (boolean && name == "Toybox.Lang.Boolean")
        }
        Type::Union(types) => types.iter().all(|ty| string_operand(ty, boolean)),
        Type::Void => false,
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
            ValueType::NullableInteger,
            ValueType::String,
            ValueType::NullableString,
            ValueType::Boolean,
            ValueType::Char,
            ValueType::Null,
            ValueType::NullableDictionary,
        ];
        for left in types {
            for right in types {
                let lhs = left.api_type();
                let rhs = right.api_type();
                for op in [BinaryOp::Add, BinaryOp::Subtract] {
                    let result = binary_result(op, &lhs, &rhs);
                    let expected = match (op, left, right) {
                        (BinaryOp::Add, ValueType::String, other)
                        | (BinaryOp::Add, other, ValueType::String)
                            if matches!(
                                other,
                                ValueType::Integer
                                    | ValueType::NullableInteger
                                    | ValueType::String
                                    | ValueType::NullableString
                                    | ValueType::Boolean
                                    | ValueType::Char
                                    | ValueType::Null
                            ) =>
                        {
                            Some(ValueType::String)
                        }
                        (_, ValueType::Integer, ValueType::Integer) => Some(ValueType::Integer),
                        (BinaryOp::Add, ValueType::Char, ValueType::Char) => {
                            Some(ValueType::String)
                        }
                        (BinaryOp::Add, ValueType::Integer, ValueType::Char)
                        | (BinaryOp::Add, ValueType::Char, ValueType::Integer) => {
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
                for op in [ComparisonOp::Equal, ComparisonOp::Greater] {
                    let result = comparison_result(op, &lhs, &rhs);
                    let null_check = op == ComparisonOp::Equal
                        && matches!(
                            (left, right),
                            (
                                ValueType::Null,
                                ValueType::Null
                                    | ValueType::NullableDictionary
                                    | ValueType::NullableInteger
                                    | ValueType::NullableString
                            ) | (
                                ValueType::NullableDictionary
                                    | ValueType::NullableInteger
                                    | ValueType::NullableString,
                                ValueType::Null
                            )
                        );
                    let strings = op == ComparisonOp::Equal
                        && matches!(
                            (left, right),
                            (
                                ValueType::String,
                                ValueType::Integer
                                    | ValueType::NullableInteger
                                    | ValueType::String
                                    | ValueType::NullableString
                                    | ValueType::Char
                                    | ValueType::Null
                            ) | (
                                ValueType::Integer
                                    | ValueType::NullableInteger
                                    | ValueType::NullableString
                                    | ValueType::Char
                                    | ValueType::Null,
                                ValueType::String
                            )
                        );
                    assert_eq!(
                        result.is_some(),
                        (matches!(left, ValueType::Integer | ValueType::Char)
                            && matches!(right, ValueType::Integer | ValueType::Char))
                            || null_check
                            || strings
                            || (op == ComparisonOp::Equal
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
            ValueType::NullableInteger,
            ValueType::String,
            ValueType::NullableString,
            ValueType::Boolean,
            ValueType::Char,
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
            None
        );
    }
}
