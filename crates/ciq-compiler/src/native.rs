//! Adapt catalogue callbacks to the supported executable parameter profile.
use crate::{ClassBase, ParameterType, ReturnType, api};

pub(crate) struct CallbackSignature {
    pub parameters: Vec<ParameterType>,
    pub result: ReturnType,
}

/// Resolve a catalogue callback into supported IR contracts. Unknown members,
/// ordinary methods, and signatures outside the current subset return None.
pub(crate) fn callback(base: ClassBase, member: &str) -> Option<CallbackSignature> {
    let class = match base {
        ClassBase::ApplicationAppBase => "Toybox.Application.AppBase",
    };
    let resolved = api::core().method(class, member)?;
    let method = resolved.method;
    if !method.callback {
        return None;
    }
    let parameters = method
        .parameters
        .iter()
        .map(|ty| {
            [ParameterType::Integer, ParameterType::NullableDictionary]
                .into_iter()
                .find(|candidate| candidate.api_type() == *ty)
        })
        .collect::<Option<Vec<_>>>()?;
    let result = [ReturnType::Integer, ReturnType::Void]
        .into_iter()
        .find(|candidate| candidate.api_type() == method.result)?;
    Some(CallbackSignature { parameters, result })
}

/// Resolve a value-call contract through the catalogue, then require its bounded
/// executable profile. API existence and supported lowering are separate checks.
pub(crate) fn value_call(
    receiver: &api::Type,
    member: &str,
    arguments: &[api::Type],
) -> Result<api::Type, &'static str> {
    let api::Type::Named(class) = receiver else {
        return Err("native calls require a supported class-valued receiver");
    };
    let resolved = api::core()
        .method(class, member)
        .ok_or("native member is not known in the API catalogue")?;
    if resolved.method.parameters.len() != arguments.len() {
        return Err("native call argument count does not match its API contract");
    }
    if resolved.method.parameters != arguments {
        return Err("native call argument types do not match its API contract");
    }
    if class != "Toybox.Lang.Number"
        || !matches!(
            (resolved.declaring_class.name.as_str(), member),
            ("Toybox.Lang.Object", "toString") | ("Toybox.Lang.Number", "format")
        )
    {
        return Err("native member has no supported lowering for this receiver");
    }
    Ok(resolved.method.result.clone())
}
