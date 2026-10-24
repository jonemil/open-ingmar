//! Structural Symbol-keyed Dictionary contracts. Every named field is optional.
use crate::api::Type;
use std::collections::BTreeMap;

/// Validate a finite record schema under the shared value/depth limits.
pub(crate) fn validate(fields: &BTreeMap<String, Type>) -> Result<(), &'static str> {
    if fields.len() > 256 {
        return Err("record exceeds 256 fields");
    }
    for (name, ty) in fields {
        let mut chars = name.chars();
        if !chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err("record keys require Symbol identifiers");
        }
        if !crate::structural::type_depth(ty, 1) {
            return Err("record exceeds 64 levels");
        }
        if *ty == Type::named("Toybox.Graphics.Dc") || crate::unions::result(ty.clone()).is_none() {
            return Err("unsupported record field contract");
        }
    }
    Ok(())
}

/// Read an optional named field; unknown keys retain the dynamic value contract.
pub(crate) fn read(fields: &BTreeMap<String, Type>, key: Option<&str>) -> Type {
    key.and_then(|key| fields.get(key))
        .cloned()
        .map(|ty| crate::unions::normalize(vec![ty, Type::Null]))
        .unwrap_or(Type::Any)
}

pub(crate) fn refine_result(
    receiver: &Type,
    member: &str,
    key: Option<&str>,
    result: Type,
) -> Type {
    if let (Type::Options(fields), "get") = (receiver, member) {
        read(fields, key)
    } else {
        result
    }
}

pub(crate) fn tag(fields: &BTreeMap<String, Type>) -> String {
    format!(
        "options({})",
        fields
            .iter()
            .map(|(key, ty)| format!(
                "{key}={}",
                crate::type_text::result_tag(
                    &crate::unions::result(ty.clone()).expect("validated record field")
                )
            ))
            .collect::<Vec<_>>()
            .join(";")
    )
}

pub(crate) fn parse(tag: &str) -> Result<BTreeMap<String, Type>, &'static str> {
    let body = tag
        .strip_prefix("options(")
        .and_then(|s| s.strip_suffix(')'))
        .ok_or("invalid record contract")?;
    let mut fields = BTreeMap::new();
    for field in if body.is_empty() {
        vec![]
    } else {
        crate::structural::split(body, ';')?
    } {
        let (key, value) = field
            .split_once('=')
            .ok_or("record field requires a type")?;
        let value = crate::type_text::call_result(value)?;
        if fields
            .insert(key.to_owned(), crate::tuples::exact_type(&value))
            .is_some()
        {
            return Err("duplicate record key");
        }
    }
    validate(&fields)?;
    Ok(fields)
}
