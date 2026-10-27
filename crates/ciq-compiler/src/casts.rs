//! Lexical value assertions; no runtime conversion or check.
use crate::{ValueType, api::Type};
use std::borrow::Cow;

/// Project an asserted contract to the executable value representation, retaining
/// exact source identities separately through the assertion's saved target.
pub(crate) fn value_type(ty: &Type) -> Option<ValueType> {
    crate::recursive::validate(ty).ok()?;
    if *ty == Type::Null || *ty == Type::Void || *ty == Type::named("Toybox.Graphics.Dc") {
        return None;
    }
    ValueType::from_api_type(ty).or_else(|| {
        crate::unions::result(ty.clone()).and_then(|result| crate::call_result_type(result).ok())
    })
}

/// Print an executable assertion target using shared signature spellings.
/// Void, Null alone, unresolved classes, and unsupported value shapes are errors.
pub(crate) fn type_tag(ty: &Type) -> Option<Cow<'static, str>> {
    value_type(ty)?;
    let parameter = crate::unions::parameter(ty.clone())?;
    Some(crate::type_text::parameter_tag(&parameter))
}

/// Parse a saved assertion without erasing nested or union source identities.
pub(crate) fn parse_type(tag: &str) -> Option<Type> {
    let parameter = crate::type_text::parameter(tag).ok()?;
    let ty = parameter
        .source_contract()
        .map(|contract| Type::SourceValue(Box::new(contract)))
        .unwrap_or_else(|| parameter.api_type());
    type_tag(&ty)?;
    Some(ty)
}
