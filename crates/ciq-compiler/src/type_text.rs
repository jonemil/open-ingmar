//! Shared saved-type spellings with explicit per-consumer acceptance rules.
use crate::{
    ArrayElement, InterfaceType, NativeClass, NumericAlias, ParameterType, ReturnType, SourceClass,
    api::Type,
};
use std::borrow::Cow;

// Canonical spellings common to ordinary parameter and call-result contracts.
const NAMED: &[(&str, ParameterType, ReturnType)] = &[
    (
        "app-base",
        ParameterType::Object(NativeClass::AppBase),
        ReturnType::Object(NativeClass::AppBase),
    ),
    ("any", ParameterType::Any, ReturnType::Any),
    ("null", ParameterType::Null, ReturnType::Null),
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
    ArrayElement::Null,
    ArrayElement::Any,
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
    if tag.starts_with("rec(") || tag.starts_with("ref(") {
        if let ParameterType::Recursive(ty) = parameter(tag).ok()? {
            return Some(ArrayElement::Recursive(ty));
        }
    }

    if let Some(body) = tag.strip_prefix("exact(").and_then(|s| s.strip_suffix(')')) {
        crate::structural::split(tag, '\0').ok()?;
        return Some(ArrayElement::Source(Box::new(
            call_result(body).ok()?.source_contract()?,
        )));
    }
    let nested = tag
        .strip_prefix("nested(")
        .map(|body| (body, false))
        .or_else(|| {
            tag.strip_prefix("nullable-nested(")
                .map(|body| (body, true))
        });
    if let Some((body, nullable)) = nested {
        crate::structural::split(tag, '\0').ok()?;
        let ty = call_result(body.strip_suffix(')')?).ok()?.api_type();
        let element = ArrayElement::from_api_type(&if nullable { ty.with_null() } else { ty })?;
        return matches!(element, ArrayElement::Nested { .. }).then_some(element);
    }
    if tag.starts_with("interface(") || tag.starts_with("union(") || tag.starts_with("callback(") {
        return ArrayElement::from_api_type(&call_result(tag).ok()?.api_type());
    }
    ARRAY_ELEMENTS.iter().find(|e| e.text() == tag).cloned()
}

/// Parse the ordinary call-result domain, retaining exact source identities.
/// Specialized headers, parameters, joins, and casts narrow it separately.
pub(super) fn call_result(tag: &str) -> Result<ReturnType, &'static str> {
    if tag.starts_with("rec(") || tag.starts_with("ref(") {
        crate::structural::split(tag, '\0')?;
    }

    if let Some(body) = tag.strip_prefix("rec(").and_then(|s| s.strip_suffix(')')) {
        return Ok(ReturnType::Recursive(Type::Recursive(Box::new(
            parameter(body)?.exact_type(),
        ))));
    }
    if let Some(index) = tag.strip_prefix("ref(").and_then(|s| s.strip_suffix(')')) {
        return Ok(ReturnType::Recursive(Type::RecursionRef(
            index.parse().map_err(|_| "invalid recursive reference")?,
        )));
    }

    if tag.starts_with("options(") {
        return crate::records::parse(tag).map(ReturnType::Options);
    }
    if tag.starts_with("tuple(") {
        return crate::tuples::parse(tag).map(ReturnType::Tuple);
    }
    if (tag.starts_with("interface(") || tag.starts_with("union(") || tag.starts_with("callback("))
        && tag.ends_with("-array")
    {
        return element(tag.strip_suffix("-array").unwrap())
            .map(ReturnType::Array)
            .ok_or("unsupported array element contract");
    }
    if tag.starts_with("callback(") {
        return crate::callbacks::parse(tag).map(|s| ReturnType::Callback(Box::new(s)));
    }
    if tag.starts_with("interface(") {
        return crate::structural::parse(tag).map(|shape| ReturnType::Structural(Box::new(shape)));
    }
    if let Some(body) = tag
        .strip_prefix("source-union(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let parts = crate::structural::split(body, '|')?;
        if !(2..=crate::MAX_PARAMETERS).contains(&parts.len()) {
            return Err("invalid source union alternatives");
        }
        let mut classes = Vec::new();
        let mut native = Vec::new();
        for part in parts {
            if (part.starts_with("source-union(") || part.starts_with("union("))
                && part.ends_with(')')
            {
                return Err("nested source union tags are not supported");
            }
            match call_result(part)? {
                ReturnType::SourceObject(class) => classes.push(class),
                ReturnType::SourceUnion(_) | ReturnType::NullableSourceObject(_) => {
                    return Err("source union requires atomic alternatives");
                }
                other => native.push(other.api_type()),
            }
        }
        return crate::SourceUnion::with_native(classes, native, false)
            .map(crate::SourceUnion::result);
    }
    if tag.starts_with("union(") {
        let body = tag
            .strip_prefix("union(")
            .and_then(|s| s.strip_suffix(')'))
            .ok_or("invalid union tag")?;
        let parts = crate::structural::split(body, '|')?;
        if parts.len() < 2
            || parts.len() > crate::MAX_PARAMETERS
            || parts.iter().any(|p| {
                p.ends_with(')')
                    && (p.starts_with("union(")
                        || p.starts_with("source-union(")
                        || p.starts_with("source(")
                        || p.starts_with("nullable-source("))
            })
        {
            return Err("invalid native union alternatives");
        }
        let types = parts
            .iter()
            .map(|p| call_result(p).map(|r| r.api_type()))
            .collect::<Result<Vec<_>, _>>()?;
        return crate::unions::result(crate::unions::normalize(types))
            .ok_or("unsupported union contract");
    }
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
    NAMED
        .iter()
        .find(|(name, _, _)| *name == tag)
        .map(|(_, _, r)| r.clone())
        .ok_or("unsupported call result type")
}

/// Parse value parameter metadata, including tuples and nullable Dictionaries.
/// Void is not a value parameter contract.
pub(super) fn parameter(tag: &str) -> Result<ParameterType, &'static str> {
    if tag.starts_with("rec(") || tag.starts_with("ref(") {
        crate::structural::split(tag, '\0')?;
    }

    if let Some(body) = tag.strip_prefix("rec(").and_then(|s| s.strip_suffix(')')) {
        return Ok(ParameterType::Recursive(Type::Recursive(Box::new(
            parameter(body)?.exact_type(),
        ))));
    }
    if let Some(index) = tag.strip_prefix("ref(").and_then(|s| s.strip_suffix(')')) {
        return Ok(ParameterType::Recursive(Type::RecursionRef(
            index.parse().map_err(|_| "invalid recursive reference")?,
        )));
    }

    if tag == "nullable-dictionary" {
        return Ok(ParameterType::NullableDictionary);
    }
    if let Some((_, parameter, _)) = NAMED.iter().find(|(name, _, _)| *name == tag) {
        return Ok(parameter.clone());
    }
    match call_result(tag) {
        Ok(ReturnType::Options(fields)) => Ok(ParameterType::Options(fields)),
        Ok(ReturnType::Tuple(tuple)) => Ok(ParameterType::Tuple(tuple)),
        Ok(ReturnType::Structural(shape)) => Ok(ParameterType::Structural(shape)),
        Ok(ReturnType::Callback(signature)) => Ok(ParameterType::Callback(signature)),
        Ok(ReturnType::SourceUnion(contract)) => Ok(contract.parameter()),
        Ok(ReturnType::SourceObject(class)) => Ok(ParameterType::SourceObject(class)),
        Ok(ReturnType::NullableSourceObject(class)) => {
            Ok(ParameterType::NullableSourceObject(class))
        }
        Ok(ReturnType::TypedDictionary(ty)) => Ok(ParameterType::TypedDictionary(ty)),
        Ok(ReturnType::Array(element)) => Ok(ParameterType::Array(element)),
        Ok(ReturnType::Union(ty)) => Ok(ParameterType::Union(ty)),
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
    if let ReturnType::Recursive(ty) = ty {
        return Cow::Owned(crate::recursive::tag(ty));
    }

    if let ReturnType::Options(fields) = ty {
        return Cow::Owned(crate::records::tag(fields));
    }
    match ty {
        ReturnType::Structural(shape) => Cow::Owned(shape.tag()),
        ReturnType::Callback(signature) => Cow::Owned(crate::callbacks::tag(signature)),
        ReturnType::Union(ty) => Cow::Owned(crate::unions::text(ty)),
        ReturnType::Array(element) => Cow::Owned(format!("{}-array", element.text())),
        ReturnType::TypedDictionary(ty) => Cow::Owned(ty.text()),
        ReturnType::SourceUnion(contract) => Cow::Owned(contract.text()),
        ReturnType::SourceObject(class) => Cow::Owned(format!("source({class})")),
        ReturnType::NullableSourceObject(class) => Cow::Owned(format!("nullable-source({class})")),
        ReturnType::Tuple(tuple) => Cow::Owned(tuple.tag()),
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
    if let ParameterType::Recursive(ty) = ty {
        return Cow::Owned(crate::recursive::tag(ty));
    }

    if let ParameterType::Options(fields) = ty {
        return Cow::Owned(crate::records::tag(fields));
    }
    match ty {
        ParameterType::Tuple(tuple) => Cow::Owned(tuple.tag()),
        ParameterType::Structural(shape) => Cow::Owned(shape.tag()),
        ParameterType::Callback(signature) => Cow::Owned(crate::callbacks::tag(signature)),
        ParameterType::Union(ty) => Cow::Owned(crate::unions::text(ty)),
        ParameterType::Array(element) => Cow::Owned(format!("{}-array", element.text())),
        ParameterType::TypedDictionary(ty) => Cow::Owned(ty.text()),
        ParameterType::SourceUnion(contract) => Cow::Owned(contract.text()),
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
    if tag.starts_with("options(") {
        return crate::records::parse(tag).ok().map(Type::Options);
    }
    if tag == "null" {
        return Some(Type::Null);
    }
    if let Some(element) = tag.strip_prefix("array-").and_then(element) {
        return Some(Type::Array(Box::new(element.api_type())));
    }
    match call_result(tag).ok()? {
        ReturnType::Dictionary
        | ReturnType::SourceUnion(_)
        | ReturnType::SourceObject(_)
        | ReturnType::NullableSourceObject(_)
        | ReturnType::Object(_)
        | ReturnType::Array(_)
        | ReturnType::Void => None,
        ty => Some(ty.api_type()),
    }
}

/// Print a supported join type without broadening its accepted domain.
pub(super) fn join_tag(ty: &Type) -> Option<Cow<'static, str>> {
    if let Type::Options(fields) = ty {
        return Some(Cow::Owned(crate::records::tag(fields)));
    }
    if let Type::TupleValue(tuple) = ty {
        return Some(Cow::Owned(tuple.tag()));
    }
    if matches!(ty, Type::Union(_)) && named_result(ty).is_none() {
        return Some(Cow::Owned(crate::unions::text(ty)));
    }
    if *ty == Type::Null {
        return Some(Cow::Borrowed("null"));
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

/// Parse a local storage contract, including value domains excluded from joins.
pub(super) fn local_contract(tag: &str) -> Option<crate::JoinType> {
    crate::joins::parse_contract(tag).or_else(|| {
        let ty = call_result(tag).ok()?.api_type();
        crate::ValueType::from_api_type(&ty).map(|_| crate::JoinType::Api(ty))
    })
}

/// Print a local storage contract without widening conditional-join syntax.
pub(super) fn local_contract_tag(ty: &crate::JoinType) -> Option<Cow<'static, str>> {
    crate::joins::contract_tag(ty).or_else(|| {
        let api = ty.api_type();
        NAMED
            .iter()
            .find_map(|(tag, _, result)| (result.api_type() == api).then_some(Cow::Borrowed(*tag)))
    })
}

pub(crate) fn named_parameter(ty: &Type) -> Option<ParameterType> {
    NAMED
        .iter()
        .find(|(_, p, _)| p.api_type() == *ty)
        .map(|(_, p, _)| p.clone())
}
pub(crate) fn named_result(ty: &Type) -> Option<ReturnType> {
    NAMED
        .iter()
        .find(|(_, _, r)| r.api_type() == *ty)
        .map(|(_, _, r)| r.clone())
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
                [true, true, true, true, true],
            ),
            ("nullable-dictionary", [true, false, false, false, true]),
            (
                "source(Counter@Lang.Object)",
                [true, true, true, false, true],
            ),
            (
                "nullable-source(Counter@Lang.Object)",
                [true, true, true, false, true],
            ),
            ("number-array", [true, true, true, false, true]),
            ("array-number", [false, false, false, true, false]),
            ("dc-array", [false; 5]),
            ("comparable", [true, true, true, true, true]),
            ("view", [true, true, true, false, true]),
            ("tuple(view)", [true, true, true, true, true]),
            ("options(offset=integer)", [true, true, true, true, true]),
            ("null", [true, true, true, true, false]),
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
            assert_eq!(element(&tag), Some(e.clone()));
            assert_eq!(
                call_result(&format!("{tag}-array")),
                Ok(ReturnType::Array(e.clone()))
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
            function_result("source(Counter@Application.AppBase)")
                .map(|r| result_tag(&r).into_owned()),
            Ok("source(Counter@Application.AppBase)".into())
        );
        assert_eq!(
            parameter("dictionary(string=>)"),
            Err("unsupported Dictionary contract")
        );
    }
}
