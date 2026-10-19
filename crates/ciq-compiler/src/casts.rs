//! Supported lexical type assertions; no runtime conversion or check.
use crate::{ValueType, api::Type};

/// Return the saved cast tag for a supported scalar or opaque reference type.
/// Parameterized containers, source classes, interfaces, and Null alone require
/// other profiles.
pub(crate) fn type_tag(ty: &Type) -> Option<&'static str> {
    if ty == &Type::named("Toybox.Lang.Dictionary") {
        return Some("dictionary");
    }
    let value = ValueType::from_api_type(ty)?;
    if matches!(
        value,
        ValueType::Null
            | ValueType::ByteArrayOptions
            | ValueType::Array(_)
            | ValueType::TypedDictionary(_)
            | ValueType::Object(_)
            | ValueType::Tuple(_)
            | ValueType::Interface(_)
            | ValueType::NullableInterface(_)
    ) {
        return None;
    }
    match crate::joins::type_tag(ty)? {
        std::borrow::Cow::Borrowed(tag) => Some(tag),
        std::borrow::Cow::Owned(_) => None,
    }
}

/// Parse a cast target tag, rejecting targets outside the lexical-cast profile.
pub(crate) fn parse_type(tag: &str) -> Option<Type> {
    let ty = if tag == "dictionary" {
        Type::named("Toybox.Lang.Dictionary")
    } else {
        crate::joins::parse_type(tag)?
    };
    type_tag(&ty)?;
    Some(ty)
}
