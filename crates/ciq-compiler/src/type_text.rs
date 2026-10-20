//! Shared saved-type spellings with explicit per-consumer acceptance rules.
use crate::{
    ArrayElement, InterfaceType, NativeClass, NumericAlias, ParameterType, ReturnType, SourceClass,
    api::Type,
};
use std::borrow::Cow;

// Canonical spellings common to ordinary parameter and call-result contracts.
const NAMED: &[(&str, ParameterType, ReturnType)] = &[
    ("integer", ParameterType::Integer, ReturnType::Integer),
    ("long", ParameterType::Long, ReturnType::Long),
    ("float", ParameterType::Float, ReturnType::Float),
    ("double", ParameterType::Double, ReturnType::Double),
    ("string", ParameterType::String, ReturnType::String),
    ("boolean", ParameterType::Boolean, ReturnType::Boolean),
    ("char", ParameterType::Char, ReturnType::Char),
    (
        "dictionary",
        ParameterType::Dictionary,
        ReturnType::Dictionary,
    ),
    ("object", ParameterType::LangObject, ReturnType::LangObject),
    (
        "nullable-object",
        ParameterType::NullableObject,
        ReturnType::NullableObject,
    ),
    (
        "weak-reference",
        ParameterType::WeakReference,
        ReturnType::WeakReference,
    ),
    (
        "nullable-weak-reference",
        ParameterType::NullableWeakReference,
        ReturnType::NullableWeakReference,
    ),
    (
        "byte-array",
        ParameterType::ByteArray,
        ReturnType::ByteArray,
    ),
    (
        "nullable-byte-array",
        ParameterType::NullableByteArray,
        ReturnType::NullableByteArray,
    ),
    ("method", ParameterType::Method, ReturnType::Method),
    (
        "nullable-method",
        ParameterType::NullableMethod,
        ReturnType::NullableMethod,
    ),
    ("symbol", ParameterType::Symbol, ReturnType::Symbol),
    (
        "nullable-symbol",
        ParameterType::NullableSymbol,
        ReturnType::NullableSymbol,
    ),
    ("numeric", ParameterType::Numeric, ReturnType::Numeric),
    (
        "integer-union",
        ParameterType::IntegerAlias,
        ReturnType::IntegerAlias,
    ),
    ("decimal", ParameterType::Decimal, ReturnType::Decimal),
    (
        "nullable-numeric",
        ParameterType::NullableNumeric,
        ReturnType::NullableNumeric,
    ),
    (
        "nullable-integer-union",
        ParameterType::NullableIntegerAlias,
        ReturnType::NullableIntegerAlias,
    ),
    (
        "nullable-decimal",
        ParameterType::NullableDecimal,
        ReturnType::NullableDecimal,
    ),
    (
        "number-format",
        ParameterType::NumberFormat,
        ReturnType::NumberFormat,
    ),
    ("endian", ParameterType::Endian, ReturnType::Endian),
    (
        "nullable-integer",
        ParameterType::NullableInteger,
        ReturnType::NullableInteger,
    ),
    (
        "nullable-long",
        ParameterType::NullableLong,
        ReturnType::NullableLong,
    ),
    (
        "nullable-float",
        ParameterType::NullableFloat,
        ReturnType::NullableFloat,
    ),
    (
        "nullable-double",
        ParameterType::NullableDouble,
        ReturnType::NullableDouble,
    ),
    (
        "nullable-string",
        ParameterType::NullableString,
        ReturnType::NullableString,
    ),
    (
        "nullable-boolean",
        ParameterType::NullableBoolean,
        ReturnType::NullableBoolean,
    ),
    (
        "nullable-char",
        ParameterType::NullableChar,
        ReturnType::NullableChar,
    ),
    (
        "comparable",
        ParameterType::Interface(InterfaceType::Comparable),
        ReturnType::Interface(InterfaceType::Comparable),
    ),
    (
        "comparator",
        ParameterType::Interface(InterfaceType::Comparator),
        ReturnType::Interface(InterfaceType::Comparator),
    ),
    (
        "nullable-comparable",
        ParameterType::NullableInterface(InterfaceType::Comparable),
        ReturnType::NullableInterface(InterfaceType::Comparable),
    ),
    (
        "nullable-comparator",
        ParameterType::NullableInterface(InterfaceType::Comparator),
        ReturnType::NullableInterface(InterfaceType::Comparator),
    ),
    (
        "view",
        ParameterType::Object(NativeClass::View),
        ReturnType::Object(NativeClass::View),
    ),
    (
        "dc",
        ParameterType::Object(NativeClass::Dc),
        ReturnType::Object(NativeClass::Dc),
    ),
    (
        "behavior-delegate",
        ParameterType::Object(NativeClass::BehaviorDelegate),
        ReturnType::Object(NativeClass::BehaviorDelegate),
    ),
];

pub(super) const ARRAY_ELEMENTS: &[ArrayElement] = &[
    ArrayElement::LangObject,
    ArrayElement::NullableObject,
    ArrayElement::Interface(InterfaceType::Comparable),
    ArrayElement::Interface(InterfaceType::Comparator),
    ArrayElement::NullableInterface(InterfaceType::Comparable),
    ArrayElement::NullableInterface(InterfaceType::Comparator),
    ArrayElement::Number,
    ArrayElement::NullableNumber,
    ArrayElement::Double,
    ArrayElement::NullableDouble,
    ArrayElement::Float,
    ArrayElement::NullableFloat,
    ArrayElement::Long,
    ArrayElement::NullableLong,
    ArrayElement::NullableChar,
    ArrayElement::NullableBoolean,
    ArrayElement::Symbol,
    ArrayElement::NullableSymbol,
    ArrayElement::Method,
    ArrayElement::NullableMethod,
    ArrayElement::WeakReference,
    ArrayElement::NullableWeakReference,
    ArrayElement::ByteArray,
    ArrayElement::NullableByteArray,
    ArrayElement::Char,
    ArrayElement::Boolean,
    ArrayElement::String,
    ArrayElement::NullableString,
    ArrayElement::Object(NativeClass::View),
    ArrayElement::Numeric {
        alias: NumericAlias::Integer,
        nullable: false,
    },
    ArrayElement::Numeric {
        alias: NumericAlias::Integer,
        nullable: true,
    },
    ArrayElement::Numeric {
        alias: NumericAlias::Decimal,
        nullable: false,
    },
    ArrayElement::Numeric {
        alias: NumericAlias::Decimal,
        nullable: true,
    },
    ArrayElement::Numeric {
        alias: NumericAlias::Numeric,
        nullable: false,
    },
    ArrayElement::Numeric {
        alias: NumericAlias::Numeric,
        nullable: true,
    },
];

/// Parse an Array element tag; admissible contents are checked by its consumer.
pub(super) fn element(tag: &str) -> Option<ArrayElement> {
    ARRAY_ELEMENTS.iter().copied().find(|e| e.text() == tag)
}

/// Parse the ordinary call-result domain, retaining exact source identities.
/// Specialized headers, parameters, joins, and casts narrow it separately.
pub(super) fn call_result(tag: &str) -> Result<ReturnType, &'static str> {
    if tag.starts_with("nullable-source(") && tag.ends_with(')') {
        return SourceClass::parse(&tag[16..tag.len() - 1])
            .map(ReturnType::NullableSourceObject)
            .ok_or("invalid source reference contract");
    }
    if tag.starts_with("source(") && tag.ends_with(')') {
        return SourceClass::parse(&tag[7..tag.len() - 1])
            .map(ReturnType::SourceObject)
            .ok_or("invalid source reference contract");
    }
    if tag.starts_with("dictionary(") {
        return crate::DictionaryType::parse(tag)
            .map(ReturnType::TypedDictionary)
            .ok_or("unsupported Dictionary contract");
    }
    if let Some(element) = tag.strip_suffix("-array").and_then(element) {
        return Ok(ReturnType::Array(element));
    }
    match tag {
        "one-view" => Ok(ReturnType::Tuple(crate::TupleShape::OneView)),
        "view-delegate" => Ok(ReturnType::Tuple(crate::TupleShape::ViewDelegate)),
        _ => NAMED
            .iter()
            .find(|(name, _, _)| *name == tag)
            .map(|(_, _, r)| r.clone())
            .ok_or("unsupported call result type"),
    }
}

/// Parse parameter metadata, including nullable opaque Dictionaries, but not tuples/Void.
pub(super) fn parameter(tag: &str) -> Result<ParameterType, &'static str> {
    if tag == "nullable-dictionary" {
        return Ok(ParameterType::NullableDictionary);
    }
    if let Some((_, parameter, _)) = NAMED.iter().find(|(name, _, _)| *name == tag) {
        return Ok(parameter.clone());
    }
    match call_result(tag) {
        Ok(ReturnType::SourceObject(class)) => Ok(ParameterType::SourceObject(class)),
        Ok(ReturnType::NullableSourceObject(class)) => {
            Ok(ParameterType::NullableSourceObject(class))
        }
        Ok(ReturnType::TypedDictionary(ty)) => Ok(ParameterType::TypedDictionary(ty)),
        Ok(ReturnType::Array(element)) => Ok(ParameterType::Array(element)),
        Err(
            message @ ("invalid source reference contract" | "unsupported Dictionary contract"),
        ) => Err(message),
        _ => Err("unsupported parameter type"),
    }
}

/// Report whether returns alone cannot reconstruct this declared result contract.
/// Functions without value returns must also preserve non-Void results explicitly.
pub(super) fn explicit_function_result(ty: &ReturnType) -> bool {
    !matches!(
        ty,
        ReturnType::Integer
            | ReturnType::Long
            | ReturnType::Double
            | ReturnType::Float
            | ReturnType::String
            | ReturnType::Boolean
            | ReturnType::Char
            | ReturnType::Object(_)
            | ReturnType::Array(_)
            | ReturnType::Tuple(_)
            | ReturnType::Void
    )
}

/// Parse a non-Void declared function result, preserving header diagnostics.
/// Void remains inferred from void returns or the absence of a value contract.
pub(super) fn function_result(tag: &str) -> Result<ReturnType, &'static str> {
    match call_result(tag) {
        Ok(ty) => Ok(ty),
        Err(
            message @ ("invalid source reference contract" | "unsupported Dictionary contract"),
        ) => Err(message),
        _ => Err("unsupported function result contract"),
    }
}

/// Print a call-result tag without adding metadata keys or erasing source identity.
pub(super) fn result_tag(ty: &ReturnType) -> Cow<'static, str> {
    match ty {
        ReturnType::Array(element) => Cow::Owned(format!("{}-array", element.text())),
        ReturnType::TypedDictionary(ty) => Cow::Owned(ty.text()),
        ReturnType::SourceObject(class) => Cow::Owned(format!("source({class})")),
        ReturnType::NullableSourceObject(class) => Cow::Owned(format!("nullable-source({class})")),
        ReturnType::Tuple(crate::TupleShape::OneView) => Cow::Borrowed("one-view"),
        ReturnType::Tuple(crate::TupleShape::ViewDelegate) => Cow::Borrowed("view-delegate"),
        ReturnType::Void => unreachable!("Void has no value-result tag"),
        _ => Cow::Borrowed(
            NAMED
                .iter()
                .find(|(_, _, r)| r == ty)
                .expect("every named result has a saved spelling")
                .0,
        ),
    }
}

/// Print parameter metadata using the same spellings as its reader.
pub(super) fn parameter_tag(ty: &ParameterType) -> Cow<'static, str> {
    match ty {
        ParameterType::Array(element) => Cow::Owned(format!("{}-array", element.text())),
        ParameterType::TypedDictionary(ty) => Cow::Owned(ty.text()),
        ParameterType::SourceObject(class) => Cow::Owned(format!("source({class})")),
        ParameterType::NullableSourceObject(class) => {
            Cow::Owned(format!("nullable-source({class})"))
        }
        ParameterType::NullableDictionary => Cow::Borrowed("nullable-dictionary"),
        _ => Cow::Borrowed(
            NAMED
                .iter()
                .find(|(_, p, _)| p == ty)
                .expect("every named parameter has a saved spelling")
                .0,
        ),
    }
}

/// Parse an API join tag, retaining its prefix notation; source joins use `JoinType`.
pub(super) fn join(tag: &str) -> Option<Type> {
    if tag == "null" {
        return Some(Type::Null);
    }
    if tag == "byte-options" {
        return Some(Type::byte_options());
    }
    if let Some(element) = tag.strip_prefix("array-").and_then(element) {
        return Some(Type::Array(Box::new(element.api_type())));
    }
    match call_result(tag).ok()? {
        ReturnType::Dictionary
        | ReturnType::SourceObject(_)
        | ReturnType::NullableSourceObject(_)
        | ReturnType::Object(_)
        | ReturnType::Tuple(_)
        | ReturnType::Array(_)
        | ReturnType::Void => None,
        ty => Some(ty.api_type()),
    }
}

/// Print a supported join type without broadening its accepted domain.
pub(super) fn join_tag(ty: &Type) -> Option<Cow<'static, str>> {
    if *ty == Type::Null {
        return Some(Cow::Borrowed("null"));
    }
    if *ty == Type::byte_options() {
        return Some(Cow::Borrowed("byte-options"));
    }
    if let Some(dictionary) = crate::DictionaryType::from_api_type(ty) {
        return Some(Cow::Owned(dictionary.text()));
    }
    if let Type::Array(element) = ty {
        return ArrayElement::from_api_type(element)
            .map(|e| Cow::Owned(format!("array-{}", e.text())));
    }
    NAMED.iter().find_map(|(tag, _, r)| {
        (r.api_type() == *ty && join(tag).is_some()).then_some(Cow::Borrowed(*tag))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_type_contexts_keep_distinct_domains() {
        // parameter, declared function result, call result, API join, native cast
        for (tag, expected) in [
            ("integer", [true, true, true, true, true]),
            ("nullable-integer", [true, true, true, true, true]),
            ("numeric", [true, true, true, true, true]),
            ("nullable-decimal", [true, true, true, true, true]),
            ("dictionary", [true, true, true, false, true]),
            (
                "dictionary(string=>nullable-object)",
                [true, true, true, true, false],
            ),
            ("nullable-dictionary", [true, false, false, false, false]),
            (
                "source(Counter@Lang.Object)",
                [true, true, true, false, false],
            ),
            (
                "nullable-source(Counter@Lang.Object)",
                [true, true, true, false, false],
            ),
            ("number-array", [true, true, true, false, false]),
            ("array-number", [false, false, false, true, false]),
            ("dc-array", [false; 5]),
            ("comparable", [true, true, true, true, false]),
            ("view", [true, true, true, false, false]),
            ("one-view", [false, true, true, false, false]),
            ("byte-options", [false, false, false, true, false]),
            ("null", [false, false, false, true, false]),
            ("void", [false; 5]),
        ] {
            assert_eq!(
                [
                    parameter(tag).is_ok(),
                    function_result(tag).is_ok(),
                    call_result(tag).is_ok(),
                    join(tag).is_some(),
                    crate::casts::parse_type(tag).is_some()
                ],
                expected,
                "{tag}"
            );
        }
    }

    #[test]
    fn shared_spellings_round_trip_without_changing_array_notation() {
        for (tag, p, r) in NAMED {
            assert_eq!(parameter_tag(p), *tag);
            assert_eq!(result_tag(r), *tag);
            assert_eq!(parameter(tag).as_ref(), Ok(p));
            assert_eq!(call_result(tag).as_ref(), Ok(r));
        }
        for e in ARRAY_ELEMENTS {
            let tag = e.text();
            assert_eq!(element(tag), Some(*e));
            assert_eq!(
                call_result(&format!("{tag}-array")),
                Ok(ReturnType::Array(*e))
            );
            assert_eq!(
                join(&format!("array-{tag}")),
                Some(Type::Array(Box::new(e.api_type())))
            );
            assert_eq!(
                join_tag(&Type::Array(Box::new(e.api_type()))),
                Some(Cow::Owned(format!("array-{tag}")))
            );
        }
        assert_eq!(
            call_result("source(Counter@Lang.Object)").map(|r| result_tag(&r).into_owned()),
            Ok("source(Counter@Lang.Object)".into())
        );
        assert_eq!(
            function_result("source(Counter@Application.AppBase)"),
            Err("invalid source reference contract")
        );
        assert_eq!(
            parameter("dictionary(string=>)"),
            Err("unsupported Dictionary contract")
        );
    }
}
