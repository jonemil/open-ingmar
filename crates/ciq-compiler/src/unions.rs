//! Normalize union alternatives, retaining exact source element contracts.
use crate::{ParameterType, ReturnType, ValueType, api::Type};

/// Flatten and deduplicate alternatives, absorbing Any and preserving the
/// established numeric alias order. Exact element identities merge under the
/// source-union budget; an over-budget union remains unrepresentable by value
/// consumers rather than panicking. Null is last in ordinary native unions.
pub(crate) fn normalize(types: Vec<Type>) -> Type {
    fn flatten(ty: Type, out: &mut Vec<Type>) {
        if let Type::Union(parts) = ty {
            for part in parts {
                flatten(part, out);
            }
        } else if !out.contains(&ty) {
            out.push(ty);
        }
    }
    let mut out = Vec::new();
    for ty in types {
        flatten(ty, &mut out);
    }
    if out.contains(&Type::Void) {
        return Type::Void;
    }
    if out.contains(&Type::Any) {
        return Type::Any;
    }
    if out.iter().any(|ty| matches!(ty, Type::SourceValue(_))) {
        let original = out.clone();
        let mut classes = Vec::new();
        let mut native = Vec::new();
        let mut nullable = false;
        for ty in out {
            if let Type::SourceValue(contract) = ty {
                classes.extend(contract.classes);
                native.extend(contract.native);
                nullable |= contract.nullable;
            } else if ty == Type::Null {
                nullable = true;
            } else {
                native.push(ty);
            }
        }
        return crate::SourceUnion::with_native(classes, native, nullable)
            .map(|contract| Type::SourceValue(Box::new(contract)))
            .unwrap_or(Type::Union(original));
    }
    out.sort_by_key(|ty| {
        let rank = match ty {
            Type::Named(n) => match n.as_str() {
                "Toybox.Lang.Number" => 0,
                "Toybox.Lang.Float" => 1,
                "Toybox.Lang.Long" => 2,
                "Toybox.Lang.Double" => 3,
                _ => 4,
            },
            Type::Null => 6,
            _ => 5,
        };
        (rank, format!("{ty:?}"))
    });
    if out.len() == 1 {
        out.remove(0)
    } else {
        Type::Union(out)
    }
}

/// Adapt a normalized native type to a supported parameter contract.
pub(crate) fn parameter(ty: Type) -> Option<ParameterType> {
    if matches!(ty, Type::Recursive(_) | Type::RecursionRef(_)) {
        return Some(ParameterType::Recursive(ty));
    }

    if ty == ParameterType::NullableDictionary.api_type() {
        return Some(ParameterType::NullableDictionary);
    }
    if let Some(p) = crate::type_text::named_parameter(&ty) {
        return Some(p);
    }
    match ty {
        Type::Options(fields) => crate::records::validate(&fields)
            .ok()
            .map(|_| ParameterType::Options(fields)),
        Type::SourceValue(contract) => Some(contract.parameter()),
        Type::TupleValue(tuple) => tuple.validate().ok().map(|()| ParameterType::Tuple(tuple)),
        Type::Callback(signature) => crate::callbacks::validate(&signature)
            .ok()
            .map(|()| ParameterType::Callback(signature)),
        Type::Structural(shape) => shape
            .validate()
            .ok()
            .map(|()| ParameterType::Structural(shape)),
        Type::Null => Some(ParameterType::Null),
        Type::Array(ref e) => crate::ArrayElement::from_api_type(e).map(ParameterType::Array),
        Type::Dictionary { .. } => {
            crate::DictionaryType::from_api_type(&ty).map(ParameterType::TypedDictionary)
        }
        Type::Union(_) if ValueType::from_api_type(&ty).is_some() => Some(ParameterType::Union(ty)),
        _ => None,
    }
}
/// Adapt a normalized native type to a supported result contract.
pub(crate) fn result(ty: Type) -> Option<ReturnType> {
    if matches!(ty, Type::Recursive(_) | Type::RecursionRef(_)) {
        return Some(ReturnType::Recursive(ty));
    }

    if let Some(r) = crate::type_text::named_result(&ty) {
        return Some(r);
    }
    match ty {
        Type::Options(fields) => crate::records::validate(&fields)
            .ok()
            .map(|_| ReturnType::Options(fields)),
        Type::SourceValue(contract) => Some(contract.result()),
        Type::TupleValue(tuple) => tuple.validate().ok().map(|()| ReturnType::Tuple(tuple)),
        Type::Callback(signature) => crate::callbacks::validate(&signature)
            .ok()
            .map(|()| ReturnType::Callback(signature)),
        Type::Structural(shape) => shape
            .validate()
            .ok()
            .map(|()| ReturnType::Structural(shape)),
        Type::Null => Some(ReturnType::Null),
        Type::Array(ref e) => crate::ArrayElement::from_api_type(e).map(ReturnType::Array),
        Type::Dictionary { .. } => {
            crate::DictionaryType::from_api_type(&ty).map(ReturnType::TypedDictionary)
        }
        Type::Union(_) if ValueType::from_api_type(&ty).is_some() => Some(ReturnType::Union(ty)),
        _ => None,
    }
}

/// Serialize a normalized native union using existing atomic contract tags.
pub(crate) fn text(ty: &Type) -> String {
    let Type::Union(parts) = ty else {
        panic!("union payload must contain alternatives")
    };
    format!(
        "union({})",
        parts
            .iter()
            .map(|ty| crate::type_text::parameter_tag(
                &parameter(ty.clone()).expect("validated union alternative")
            )
            .into_owned())
            .collect::<Vec<_>>()
            .join("|")
    )
}

/// Return top-level alternatives, treating a concrete type as a singleton.
pub(crate) fn alternatives(ty: &Type) -> &[Type] {
    if let Type::Union(parts) = ty {
        parts
    } else {
        std::slice::from_ref(ty)
    }
}

/// Require a supported scalar printing profile for every alternative.
pub(crate) fn printable(ty: &Type) -> bool {
    alternatives(ty).iter().all(|part| {
        matches!(
            ValueType::from_api_type(part),
            Some(
                ValueType::Any
                    | ValueType::Integer
                    | ValueType::Long
                    | ValueType::Float
                    | ValueType::Double
                    | ValueType::Boolean
                    | ValueType::Char
                    | ValueType::String
            )
        )
    })
}

/// Check an index against every possible receiver, joining the read contracts.
pub(crate) fn index_read(receiver: &Type, key: &Type) -> Result<Type, String> {
    if let Some(unfolded) = crate::recursive::unfold(receiver) {
        return index_read(&unfolded, key);
    }

    let mut results = Vec::new();
    for part in alternatives(receiver) {
        let element = match part {
            Type::Options(fields) => crate::records::read(fields, None),
            Type::TupleValue(tuple) => {
                if !crate::native::accepts_boundary(&Type::named("Toybox.Lang.Number"), key) {
                    return Err("tuple reads require a non-null Number index".into());
                }
                tuple.read(None)?.api_type()
            }
            Type::Any => Type::Any,
            Type::Array(element) => {
                if !crate::native::accepts_boundary(&Type::named("Toybox.Lang.Number"), key) {
                    return Err("array reads require a non-null Number index".into());
                }
                (**element).clone()
            }
            Type::Named(name) if name == "Toybox.Lang.ByteArray" => {
                if !crate::native::accepts_boundary(&Type::named("Toybox.Lang.Number"), key) {
                    return Err("array reads require a non-null Number index".into());
                }
                Type::named("Toybox.Lang.Number")
            }
            Type::Dictionary { .. } => {
                crate::native::value_call(part, "get", std::slice::from_ref(key), None)?
            }
            _ => return Err("indexed union reads require non-null indexable alternatives".into()),
        };
        results.push(element);
    }
    Ok(normalize(results))
}

/// Check a write against every possible receiver without widening its elements.
pub(crate) fn index_write(receiver: &Type, key: &Type, value: &Type) -> Result<(), String> {
    if let Some(unfolded) = crate::recursive::unfold(receiver) {
        return index_write(&unfolded, key, value);
    }

    if matches!(receiver, Type::Options(_)) {
        return crate::native::value_call(receiver, "put", &[key.clone(), value.clone()], None)
            .map(|_| ());
    }
    for part in alternatives(receiver) {
        let expected = match part {
            Type::Any => Type::Any,
            Type::Array(element) => {
                index_read(part, key)?;
                (**element).clone()
            }
            Type::Named(name) if name == "Toybox.Lang.ByteArray" => {
                index_read(part, key)?;
                normalize(vec![
                    Type::named("Toybox.Lang.Number"),
                    Type::named("Toybox.Lang.Char"),
                ])
            }
            Type::Dictionary { .. } => {
                crate::native::value_call(part, "put", &[key.clone(), value.clone()], None)?;
                continue;
            }
            _ => return Err("indexed union writes require non-null indexable alternatives".into()),
        };
        if !crate::native::accepts_boundary(&expected, value) {
            return Err("indexed union write must satisfy every element contract".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_element_joins_reject_excess_alternatives_without_panicking() {
        let classes = (0..crate::MAX_PARAMETERS)
            .map(|i| crate::SourceClass::parse(&format!("C{i}@Lang.Object")).unwrap())
            .collect();
        let first = crate::SourceUnion::new(classes, false).unwrap();
        let second = crate::SourceUnion::new(
            vec![crate::SourceClass::parse("Extra@Lang.Object").unwrap()],
            false,
        )
        .unwrap();
        // Exact identities belong in the class list, never in the native
        // alternatives of a public source-union contract.
        assert!(
            crate::SourceUnion::with_native(
                first.classes().to_vec(),
                vec![Type::SourceValue(Box::new(second.clone()))],
                false,
            )
            .is_err()
        );
        let joined = normalize(vec![
            Type::SourceValue(Box::new(first)),
            Type::SourceValue(Box::new(second)),
        ]);
        assert!(crate::ArrayElement::from_api_type(&joined).is_none());
        assert!(result(Type::Array(Box::new(joined))).is_none());
    }
}
