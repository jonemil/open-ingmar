//! Operator contracts for the executable source subset, before VM selection.
//! These rules do not enumerate every operator/type combination in Monkey C.
use crate::{BinaryOp, ComparisonOp, api::Type};

/// Resolve supported arithmetic operand types to their semantic result type.
/// Only Number/Number addition and subtraction have established lowering here.
pub(crate) fn binary_result(kind: BinaryOp, left: &Type, right: &Type) -> Option<Type> {
    match kind {
        BinaryOp::Add | BinaryOp::Subtract
            if *left == Type::named("Toybox.Lang.Number") && left == right =>
        {
            Some(Type::named("Toybox.Lang.Number"))
        }
        _ => None,
    }
}

/// Resolve supported comparison types to Boolean. Number/String/Dictionary-or-null values
/// support equality with Null; Boolean pairs support equality only.
/// Other comparisons currently require Numbers.
pub(crate) fn comparison_result(kind: ComparisonOp, left: &Type, right: &Type) -> Option<Type> {
    let numbers = *left == Type::named("Toybox.Lang.Number") && left == right;
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
    (numbers || booleans || null_check).then(|| Type::named("Toybox.Lang.Boolean"))
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
            ValueType::Null,
            ValueType::NullableDictionary,
        ];
        for left in types {
            for right in types {
                let lhs = left.api_type();
                let rhs = right.api_type();
                let numbers = left == ValueType::Integer && right == ValueType::Integer;
                for op in [BinaryOp::Add, BinaryOp::Subtract] {
                    let result = binary_result(op, &lhs, &rhs);
                    assert_eq!(result.is_some(), numbers, "{op:?}: {left:?}, {right:?}");
                    if let Some(result) = result {
                        assert_eq!(result, ValueType::Integer.api_type());
                    }
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
                    assert_eq!(
                        result.is_some(),
                        numbers
                            || null_check
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
