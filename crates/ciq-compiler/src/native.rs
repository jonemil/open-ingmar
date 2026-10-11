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
    context: Option<api::ApiContext>,
) -> Result<api::Type, String> {
    let api::Type::Named(class) = receiver else {
        return Err("native calls require a supported class-valued receiver".into());
    };
    let resolved = if let Some(context) = context {
        context.method(api::core(), class, member)?
    } else {
        api::core()
            .method(class, member)
            .ok_or("native member is not known in the API catalogue")?
    };
    if resolved.method.parameters.len() != arguments.len() {
        return Err("native call argument count does not match its API contract".into());
    }
    if !resolved
        .method
        .parameters
        .iter()
        .zip(arguments)
        .all(|(expected, actual)| accepts(expected, actual))
    {
        return Err("native call argument types do not match its API contract".into());
    }
    if class == "Toybox.Lang.String"
        && member == "substring"
        && arguments
            .iter()
            .any(|ty| matches!(ty, api::Type::Null | api::Type::Union(_)))
    {
        if let Some(context) = context {
            context.require("String.substring nullable indices", api::ApiLevel(3, 3, 2))?;
        }
    }
    let supported = matches!(
        (
            class.as_str(),
            resolved.declaring_class.name.as_str(),
            member
        ),
        ("Toybox.Lang.Number", "Toybox.Lang.Object", "toString")
            | ("Toybox.Lang.Number", "Toybox.Lang.Number", "format" | "abs")
            | (
                "Toybox.Lang.String",
                "Toybox.Lang.String",
                "length" | "toLower" | "toUpper" | "toString" | "find" | "toNumber" | "substring"
            )
    );
    if !supported {
        return Err("native member has no supported lowering for this receiver".into());
    }
    Ok(resolved.method.result.clone())
}

/// Check exact types or union membership, without implicit conversions or
/// inheritance widening. Every alternative of an actual union must be accepted.
fn accepts(expected: &api::Type, actual: &api::Type) -> bool {
    if expected == actual {
        return true;
    }
    if let api::Type::Union(types) = actual {
        return types.iter().all(|ty| accepts(expected, ty));
    }
    if let api::Type::Union(types) = expected {
        return types.iter().any(|ty| ty == actual);
    }
    false
}

/// Resolve a module statement contract, then require supported Void lowering.
/// Declarations and introduction levels are checked before executable support.
pub(crate) fn module_statement(
    module: &str,
    member: &str,
    arguments: &[api::Type],
    context: Option<api::ApiContext>,
) -> Result<(), String> {
    let catalogue = api::core();
    let method = if let Some(context) = context {
        context.function(catalogue, module, member)?
    } else {
        catalogue
            .module(module)
            .and_then(|m| m.functions.iter().find(|f| f.name == member))
            .ok_or_else(|| {
                format!("native function {module}.{member} is not known in the API catalogue")
            })?
    };
    if method.parameters.len() != arguments.len() {
        return Err("native call argument count does not match its API contract".into());
    }
    if method.parameters != arguments {
        return Err("native call argument types do not match its API contract".into());
    }
    if method.result != api::Type::Void || (module, member) != ("Toybox.System", "exit") {
        return Err("native module member has no supported statement lowering".into());
    }
    Ok(())
}
