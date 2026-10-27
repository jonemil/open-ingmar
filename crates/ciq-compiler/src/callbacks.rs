//! Typed Method contracts and saved signature spelling.
use crate::{ParameterType, ReturnType, StructuralInterface, StructuralMethod};
use std::collections::BTreeMap;

/// Validate a fixed callback signature using the shared member-type restrictions.
pub(crate) fn validate(signature: &StructuralMethod) -> Result<(), String> {
    if !crate::structural::signature_depth(signature, 0) {
        return Err("callback nesting exceeds 64 levels".into());
    }
    StructuralInterface {
        declaration: None,
        fields: BTreeMap::new(),
        methods: [("invoke".into(), signature.clone())].into(),
    }
    .validate()
}

pub(crate) fn tag(signature: &StructuralMethod) -> String {
    let parameters = signature
        .parameters
        .iter()
        .map(crate::type_text::parameter_tag)
        .collect::<Vec<_>>()
        .join(";");
    let result = if signature.result == ReturnType::Void {
        "void".into()
    } else {
        crate::type_text::result_tag(&signature.result).into_owned()
    };
    format!("callback({parameters}=>{result})")
}

pub(crate) fn parse(text: &str) -> Result<StructuralMethod, &'static str> {
    let body = text
        .strip_prefix("callback(")
        .and_then(|s| s.strip_suffix(')'))
        .ok_or("invalid callback tag")?;
    let (parameters, result) = crate::structural::split_pair(body, "=>")
        .ok_or("callback parameters/result require one arrow")?;
    let parameters = if parameters.is_empty() {
        Vec::new()
    } else {
        crate::structural::split(parameters, ';')?
            .into_iter()
            .map(crate::type_text::parameter)
            .collect::<Result<Vec<ParameterType>, _>>()?
    };
    let result = if result == "void" {
        ReturnType::Void
    } else {
        crate::type_text::call_result(result)?
    };
    let signature = StructuralMethod { parameters, result };
    validate(&signature).map_err(|_| "invalid callback signature")?;
    Ok(signature)
}

pub(crate) fn value_signature(ty: &crate::api::Type) -> Option<(&StructuralMethod, bool)> {
    use crate::api::Type;
    match ty {
        Type::Callback(signature) => Some((signature, false)),
        Type::Union(parts) if parts.len() == 2 && parts.contains(&Type::Null) => {
            let Type::Callback(signature) = parts.iter().find(|ty| **ty != Type::Null)? else {
                return None;
            };
            Some((signature, true))
        }
        _ => None,
    }
}

/// Resolve a catalogue method only when it has a representable fixed signature.
/// API availability belongs to the binding, even if invocation happens elsewhere.
pub(crate) fn native_signature(
    receiver: &crate::api::Type,
    member: &str,
    context: Option<crate::api::ApiContext>,
) -> Result<StructuralMethod, String> {
    // Binding uses Array's erased declared signature; it does not run direct
    // call specialization using the captured tuple's slot types or bounds.
    let projected;
    let receiver = if crate::tuples::read_contract(receiver, None).is_some() {
        projected = crate::api::Type::Array(Box::new(
            crate::api::Type::named("Toybox.Lang.Object").with_null(),
        ));
        &projected
    } else {
        receiver
    };
    let method = crate::api::core()
        .method_for_type(receiver, member)
        .ok_or("unresolved native binding target")?;
    if method.variadic.is_some() {
        return Err("variadic binding signature is not fixed".into());
    }
    if let Some(context) = context {
        context.require(member, method.declaration.required_api)?;
    }
    StructuralMethod::from_native(method.parameters, method.result)
        .ok_or_else(|| "unsupported native binding signature".into())
}

/// Recheck a typed binding claim against every exact receiver and literal target.
/// An erased receiver cannot justify arbitrary saved signature metadata.
#[allow(clippy::too_many_arguments)]
pub(crate) fn check_binding(
    unit: &crate::CompilationUnit,
    function: &crate::Function,
    origins: &crate::source_objects::Origins,
    owner: Option<&str>,
    receiver: crate::ValueId,
    selector: crate::ValueId,
    expected: &StructuralMethod,
    context: Option<crate::api::ApiContext>,
) -> Result<(), String> {
    use crate::Operation;
    let member = function
        .operations()
        .iter()
        .find_map(|op| match &op.operation {
            Operation::Symbol { result, name } if *result == selector => Some(name.as_str()),
            _ => None,
        })
        .ok_or("typed binding requires a literal Symbol selector")?;
    let self_receiver = function
        .operations()
        .iter()
        .any(|op| matches!(op.operation,Operation::SelfValue {result, ..} if result==receiver));
    let classes = if self_receiver {
        owner.into_iter().map(str::to_owned).collect::<Vec<_>>()
    } else {
        origins
            .contract(receiver)
            .filter(|c| !c.nullable() && c.native().is_empty())
            .map(|c| c.classes().iter().map(|c| c.name.clone()).collect())
            .unwrap_or_default()
    };
    if classes.is_empty()
        && !expected.accepts(&native_signature(
            &function.value_types[&receiver].api_type(),
            member,
            context,
        )?)
    {
        return Err("bound Method signature does not match its native target".into());
    }
    for name in classes {
        let class = unit
            .classes()
            .iter()
            .find(|c| c.name == name)
            .ok_or("unknown binding receiver class")?;
        if class.methods.iter().any(|method| method.name() == "method") {
            return Err("typed binding requires the inherited Object.method implementation".into());
        }
        let actual =
            if let Some(method) = class.methods.iter().find(|method| method.name() == member) {
                StructuralMethod {
                    parameters: method.parameter_types().to_vec(),
                    result: method.return_type(),
                }
            } else {
                native_signature(
                    &crate::api::Type::named(class.base.api_name()),
                    member,
                    context,
                )?
            };
        if !expected.accepts(&actual) {
            return Err("bound Method signature does not match its source target".into());
        }
    }
    Ok(())
}
