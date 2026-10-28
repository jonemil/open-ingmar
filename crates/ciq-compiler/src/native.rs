//! Resolve catalogue contracts into supported callback and native-call profiles.
use crate::{ClassBase, ParameterType, ReturnType, api};

/// Validate the zero-argument construction profile and its initializer API level.
/// Construction yields the class reference; the initializer's Void result is discarded.
pub(crate) fn constructor(
    class: crate::NativeClass,
    context: Option<api::ApiContext>,
) -> Result<(), String> {
    if !matches!(
        class,
        crate::NativeClass::View | crate::NativeClass::BehaviorDelegate
    ) {
        return Err("native class has no supported constructor".into());
    }
    let method = if let Some(context) = context {
        context.method(api::core(), class.name(), "initialize")?
    } else {
        api::core()
            .method(class.name(), "initialize")
            .ok_or("native constructor is not declared")?
    };
    if !method.method.parameters.is_empty() || method.method.result != api::Type::Void {
        return Err("native constructor requires a zero-argument Void initializer".into());
    }
    Ok(())
}

pub(crate) struct CallbackSignature {
    pub parameters: Vec<ParameterType>,
    pub result: ReturnType,
    alternative: Option<ReturnType>,
}
impl CallbackSignature {
    pub(crate) fn accepts_result(&self, result: &ReturnType) -> bool {
        accepts(&self.result.api_type(), &result.api_type())
            || self
                .alternative
                .as_ref()
                .is_some_and(|alternative| accepts(&alternative.api_type(), &result.api_type()))
    }
}

/// Resolve a supported native constant with its API type and published value.
pub(crate) fn constant(
    module: &str,
    member: &str,
    context: Option<api::ApiContext>,
) -> Result<(api::Type, i32), String> {
    let constant = api::core()
        .constant(module, member)
        .ok_or("unsupported native constant")?;
    let ty = constant.ty.clone();
    if let Some(context) = context {
        context.check_type(api::core(), &ty)?;
        context.require(&format!("{module}.{member}"), constant.since)?;
    }
    Ok((ty, constant.value))
}

/// Resolve declared argument types for contextual source construction.
/// This substitutes container parameters without authorizing the call;
/// executable support, API availability, and actual arguments are checked later.
pub(crate) fn value_parameters(receiver: &api::Type, member: &str) -> Option<Vec<api::Type>> {
    if let Some(unfolded) = crate::recursive::unfold(receiver) {
        return value_parameters(&unfolded, member);
    }

    if matches!(receiver, api::Type::Options(_)) {
        return value_parameters(
            &api::Type::dictionary(api::Type::named("Toybox.Lang.Symbol"), api::Type::Any),
            member,
        );
    }
    if let api::Type::SourceValue(contract) = receiver {
        return value_parameters(&contract.api_type(), member);
    }
    if matches!(receiver, api::Type::TupleValue(_)) {
        return value_parameters(&api::Type::Array(Box::new(api::Type::Any)), member);
    }
    if let api::Type::Callback(signature) = receiver {
        return if member == "invoke" {
            Some(
                signature
                    .parameters
                    .iter()
                    .map(ParameterType::exact_type)
                    .collect(),
            )
        } else {
            value_parameters(&api::Type::named("Toybox.Lang.Method"), member)
        };
    }
    if let api::Type::Structural(shape) = receiver {
        return shape
            .methods
            .get(member)
            .map(|m| m.parameters.iter().map(ParameterType::exact_type).collect())
            .or_else(|| value_parameters(&api::Type::named("Toybox.Lang.Object"), member));
    }
    if let api::Type::Union(alternatives) = receiver {
        let mut signatures = alternatives
            .iter()
            .filter_map(|ty| value_parameters(ty, member));
        let first = signatures.next()?;
        return signatures
            .all(|other| {
                other.len() == first.len()
                    && first
                        .iter()
                        .zip(&other)
                        .all(|(a, b)| crate::structural::same_type(a, b))
            })
            .then_some(first);
    }
    if let api::Type::Named(name) = receiver {
        if let Some(interface) = api::core().interface(name) {
            return interface
                .methods
                .iter()
                .find(|m| m.name == member)
                .map(|m| m.parameters.clone());
        }
    }
    Some(api::core().method_for_type(receiver, member)?.parameters)
}

/// Resolve a catalogue callback into supported IR contracts. Unknown members,
/// ordinary methods, and signatures outside the current subset return None.
pub(crate) fn callback(base: ClassBase, member: &str) -> Option<CallbackSignature> {
    let class = base.api_name();
    let resolved = api::core().method(class, member)?;
    let method = resolved.method;
    if !method.callback {
        return None;
    }
    let parameters = method
        .parameters
        .iter()
        .map(|ty| {
            [
                ParameterType::Integer,
                ParameterType::NullableDictionary,
                ParameterType::Object(crate::NativeClass::Dc),
            ]
            .into_iter()
            .find(|candidate| candidate.api_type() == *ty)
        })
        .collect::<Option<Vec<_>>>()?;
    let result = [
        ReturnType::Integer,
        ReturnType::Boolean,
        ReturnType::Void,
        ReturnType::Tuple(crate::TupleType::view()),
    ]
    .into_iter()
    .find(|candidate| accepts(&method.result, &candidate.api_type()))?;
    let alternative = accepts(
        &method.result,
        &crate::TupleType::view_delegate().api_type(),
    )
    .then_some(ReturnType::Tuple(crate::TupleType::view_delegate()));
    Some(CallbackSignature {
        parameters,
        result,
        alternative,
    })
}

fn signature_call(
    method: &crate::StructuralMethod,
    arguments: &[api::Type],
    context: Option<api::ApiContext>,
) -> Result<api::Type, String> {
    for (expected, actual) in method.parameters.iter().zip(arguments) {
        structural_api(&expected.api_type(), actual, context)?;
    }
    if method.parameters.len() != arguments.len()
        || !method
            .parameters
            .iter()
            .zip(arguments)
            .all(|(expected, actual)| accepts_boundary(&expected.api_type(), actual))
    {
        return Err("call arguments do not match the declared signature".into());
    }
    Ok(method.result.api_type())
}

/// Resolve a value-call contract through the catalogue, then require its bounded
/// executable profile. Return the effective receiver-specific result; declaration
/// availability, arguments, and executable support must all pass first.
/// Comparison and equality operands follow their declared Object contracts;
/// an accepted argument may still cause a runtime type exception.
/// Any receivers defer member/argument checking to runtime and produce Any.
/// Any arguments on known receivers retain profile, arity, and API checks.
pub(crate) fn value_call(
    receiver: &api::Type,
    member: &str,
    arguments: &[api::Type],
    context: Option<api::ApiContext>,
) -> Result<api::Type, String> {
    if let Some(unfolded) = crate::recursive::unfold(receiver) {
        return value_call(&unfolded, member, arguments, context);
    }

    if let api::Type::SourceValue(contract) = receiver {
        // Source receivers retain their existing complete lookup profile;
        // native projections cannot stand in for missing source declarations.
        let projection = contract.api_type();
        return if let api::Type::Union(parts) = projection {
            let results = parts
                .iter()
                .map(|part| value_call(part, member, arguments, context))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(crate::unions::normalize(results))
        } else {
            value_call(&projection, member, arguments, context)
        };
    }
    if let api::Type::TupleValue(tuple) = receiver {
        tuple.validate()?;
        if let Some(context) = context {
            context.check_type(api::core(), receiver)?;
        }
        if member == "sort" {
            if let Some(context) = context {
                context.method(api::core(), "Toybox.Lang.Array", member)?;
            }
            let signature = api::core()
                .method_for_type(receiver, member)
                .ok_or("unresolved tuple sort")?;
            check_arguments(&signature, arguments, context)?;
            if !tuple
                .slots()
                .iter()
                .all(|slot| sort_element_supported(&slot.api_type(), arguments))
            {
                return Err("tuple sort requires non-null elements and a supported default order or non-null Comparator".into());
            }
            return Ok(api::Type::Void);
        }
        if !matches!(
            member,
            "size"
                | "isEmpty"
                | "equals"
                | "hashCode"
                | "toString"
                | "indexOf"
                | "remove"
                | "removeAll"
                | "reverse"
                | "slice"
                | "method"
                | "weak"
        ) {
            return Err("tuple method requires mutation/shape tracking not yet supported".into());
        }
        let projected = api::Type::Array(Box::new(api::Type::Any));
        let result = value_call(&projected, member, arguments, context)?;
        return if member == "reverse" {
            crate::tuples::reverse_contract(receiver).map_err(String::from)
        } else {
            Ok(result)
        };
    }
    if let api::Type::Callback(signature) = receiver {
        if let Some(context) = context {
            context.check_type(api::core(), receiver)?;
        }
        return if member == "invoke" {
            signature_call(signature, arguments, context)
        } else {
            value_call(
                &api::Type::named("Toybox.Lang.Method"),
                member,
                arguments,
                context,
            )
        };
    }
    if let api::Type::Structural(shape) = receiver {
        if let Some(context) = context {
            context.check_type(api::core(), receiver)?;
        }
        if let Some(method) = shape.methods.get(member) {
            return signature_call(method, arguments, context);
        }
        return value_call(
            &api::Type::named("Toybox.Lang.Object"),
            member,
            arguments,
            context,
        );
    }
    if *receiver == api::Type::Any {
        if arguments.len() > crate::MAX_PARAMETERS {
            return Err("call exceeds 254-argument limit".into());
        }
        crate::validate_call_names(None, member)?;
        return Ok(api::Type::Any);
    }
    if arguments.contains(&api::Type::Any) {
        if let Some(expected) = value_parameters(receiver, member) {
            if expected.len() == arguments.len() {
                let resolved = arguments
                    .iter()
                    .zip(expected)
                    .map(|(actual, expected)| {
                        if *actual == api::Type::Any {
                            expected
                        } else {
                            actual.clone()
                        }
                    })
                    .collect::<Vec<_>>();
                if !resolved.contains(&api::Type::Any) {
                    return value_call(receiver, member, &resolved, context);
                }
            }
        }
    }
    if let api::Type::Union(parts) = receiver {
        let results = parts
            .iter()
            .filter(|part| value_parameters(part, member).is_some())
            .map(|part| value_call(part, member, arguments, context))
            .collect::<Result<Vec<_>, _>>()?;
        if results.is_empty() {
            return Err("no receiver alternative declares the requested member".into());
        }
        if results.len() != parts.len() && results.iter().any(|ty| *ty != api::Type::Void) {
            // An unavailable member's runtime object can have an unmodelled
            // implementation; do not invent its return type from another part.
            return Ok(api::Type::Any);
        }
        return Ok(crate::unions::normalize(results));
    }
    if let api::Type::Named(name) = receiver {
        if let Some(interface) = crate::InterfaceType::from_name(name) {
            if let Some(context) = context {
                context.check_type(api::core(), receiver)?;
            }
            let declaration = api::core().interface(interface.name()).unwrap();
            if !declaration.methods.iter().any(|m| m.name == member) {
                // Existing interface assignability permits the Object view.
                // Keep method-specific executable restrictions in its profile.
                return value_call(
                    &api::Type::named("Toybox.Lang.Object"),
                    member,
                    arguments,
                    context,
                );
            }
            let method = declaration
                .methods
                .iter()
                .find(|m| m.name == member)
                .ok_or("member is not declared by the comparison interface")?;
            if let Some(context) = context {
                context.check_type(api::core(), receiver)?;
                context.require(&format!("{name}.{member}"), method.since)?;
            }
            if method.parameters.len() != arguments.len()
                || !method
                    .parameters
                    .iter()
                    .zip(arguments)
                    .all(|(expected, actual)| accepts_boundary(expected, actual))
            {
                return Err("interface call arguments do not match the declared signature".into());
            }
            return Ok(method.result.clone());
        }
    }
    let class = match receiver {
        api::Type::Options(fields) => {
            crate::records::validate(fields)?;
            "Toybox.Lang.Dictionary"
        }
        api::Type::Dictionary { .. }
            if crate::DictionaryType::from_api_type(receiver).is_some() =>
        {
            "Toybox.Lang.Dictionary"
        }
        api::Type::Named(class) => class.as_str(),
        api::Type::Array(element) if crate::ArrayElement::from_api_type(element).is_some() => {
            "Toybox.Lang.Array"
        }
        _ => return Err("native calls require a supported class-valued receiver".into()),
    };
    let resolved = if let Some(context) = context {
        context.check_type(api::core(), receiver)?;
        context.method(api::core(), class, member)?
    } else {
        api::core()
            .method(class, member)
            .ok_or("native member is not known in the API catalogue")?
    };
    let signature = api::core()
        .method_for_type(receiver, member)
        .ok_or("native member requires concrete container parameters")?;
    check_arguments(&signature, arguments, context)?;
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
        (class, resolved.declaring_class.name.as_str(), member),
        (
            "Toybox.Lang.Number"
                | "Toybox.Lang.Float"
                | "Toybox.Lang.Double"
                | "Toybox.Lang.Long"
                | "Toybox.Lang.String"
                | "Toybox.Lang.Symbol"
                | "Toybox.Lang.Char"
                | "Toybox.Lang.Boolean"
                | "Toybox.Lang.Array"
                | "Toybox.Lang.Dictionary"
                | "Toybox.Lang.Method"
                | "Toybox.Lang.WeakReference"
                | "Toybox.Lang.ByteArray",
            "Toybox.Lang.Object",
            "weak"
        ) | (
            "Toybox.Lang.Number"
                | "Toybox.Lang.Float"
                | "Toybox.Lang.Double"
                | "Toybox.Lang.Long"
                | "Toybox.Lang.Boolean"
                | "Toybox.Lang.Symbol"
                | "Toybox.Lang.Char"
                | "Toybox.Lang.Object"
                | "Toybox.Lang.Method"
                | "Toybox.Lang.WeakReference",
            "Toybox.Lang.Object",
            "toString" | "equals" | "hashCode"
        ) | (
            "Toybox.Lang.Method",
            "Toybox.Lang.Method",
            "invoke" | "hashCode"
        ) | (
            "Toybox.Lang.Symbol",
            "Toybox.Lang.Symbol",
            "toNumber" | "toString"
        ) | ("Toybox.Lang.Boolean", "Toybox.Lang.Boolean", "compareTo")
            | (
                "Toybox.Lang.WeakReference",
                "Toybox.Lang.WeakReference",
                "get" | "stillAlive"
            )
            | (
                "Toybox.Lang.Array" | "Toybox.Lang.Dictionary",
                "Toybox.Lang.Object",
                "equals" | "hashCode"
            )
            | (
                "Toybox.Lang.Dictionary",
                "Toybox.Lang.Dictionary",
                "size"
                    | "isEmpty"
                    | "get"
                    | "put"
                    | "hasKey"
                    | "remove"
                    | "toString"
                    | "keys"
                    | "values"
            )
            | (
                "Toybox.Lang.ByteArray",
                "Toybox.Lang.ByteArray",
                "size"
                    | "toString"
                    | "equals"
                    | "hashCode"
                    | "add"
                    | "addAll"
                    | "indexOf"
                    | "remove"
                    | "removeAll"
                    | "reverse"
                    | "slice"
                    | "decodeNumber"
                    | "encodeNumber"
            )
            | (
                "Toybox.Lang.Array",
                "Toybox.Lang.Array",
                "size"
                    | "toString"
                    | "indexOf"
                    | "add"
                    | "addAll"
                    | "remove"
                    | "removeAll"
                    | "reverse"
                    | "slice"
                    | "sort"
            )
            | (
                "Toybox.Lang.Number",
                "Toybox.Lang.Number",
                "format"
                    | "abs"
                    | "toNumber"
                    | "toLong"
                    | "toFloat"
                    | "toDouble"
                    | "toChar"
                    | "compareTo"
            )
            | (
                "Toybox.Lang.Long",
                "Toybox.Lang.Long",
                "abs"
                    | "format"
                    | "toLong"
                    | "toNumber"
                    | "toFloat"
                    | "toDouble"
                    | "toString"
                    | "equals"
                    | "compareTo"
            )
            | (
                "Toybox.Lang.Float" | "Toybox.Lang.Double",
                "Toybox.Lang.Float" | "Toybox.Lang.Double",
                "abs"
                    | "format"
                    | "toLong"
                    | "toNumber"
                    | "toFloat"
                    | "toDouble"
                    | "toString"
                    | "equals"
                    | "compareTo"
            )
            | (
                "Toybox.Lang.Char",
                "Toybox.Lang.Char",
                "toLower" | "toUpper" | "toNumber" | "toString" | "compareTo"
            )
            | (
                "Toybox.Lang.String",
                "Toybox.Lang.String",
                "length"
                    | "toLower"
                    | "toUpper"
                    | "toString"
                    | "find"
                    | "toNumber"
                    | "substring"
                    | "compareTo"
                    | "equals"
                    | "hashCode"
                    | "toNumberWithBase"
                    | "toLong"
                    | "toLongWithBase"
                    | "toFloat"
                    | "toDouble"
                    | "toCharArray"
                    | "toUtf8Array"
            )
    );
    let supported = supported
        || (class == "Toybox.Graphics.Dc" && matches!(member, "clear" | "setColor" | "drawText"));
    let supported = supported
        || (member == "method"
            && resolved.declaring_class.name == "Toybox.Lang.Object"
            && (method_receiver_profile(receiver)
                || crate::NativeClass::from_name(class).is_some())
            && !matches!(receiver, api::Type::Null | api::Type::Union(_)));
    if class == "Toybox.Lang.ByteArray"
        && member == "addAll"
        && !matches!(arguments, [api::Type::Named(name)] if name == class)
        && !matches!(arguments, [api::Type::Array(element)] if matches!(element.as_ref(), api::Type::Named(name) if name == "Toybox.Lang.Number" || name == "Toybox.Lang.Char"))
        && !matches!(arguments, [api::Type::TupleValue(tuple)] if ["Toybox.Lang.Number", "Toybox.Lang.Char"].iter().any(|name| tuple.slots().iter().all(|slot| slot.api_type() == api::Type::named(*name))))
    {
        return Err(
            "ByteArray.addAll requires ByteArray or an Array of non-null Numbers or Chars".into(),
        );
    }
    if !supported {
        return Err("native member has no supported lowering for this receiver".into());
    }
    if class == "Toybox.Lang.Array" && member == "sort" {
        if !matches!(arguments, [actual] if accepts(&crate::InterfaceType::Comparator.api_type(true), actual))
        {
            return Err("Array.sort requires a checked Comparator or Null".into());
        }
        let supported_elements = match receiver {
            api::Type::Array(element) => sort_element_supported(element, arguments),
            _ => false,
        };
        if !supported_elements {
            return Err("Array.sort requires non-null elements and a supported default order or non-null Comparator".into());
        }
    }
    if class == "Toybox.Lang.Array" && member == "slice" {
        let nullable_number = api::Type::Union(vec![
            api::Type::named("Toybox.Lang.Number"),
            api::Type::Null,
        ]);
        if arguments
            .iter()
            .any(|argument| !accepts(&nullable_number, argument))
        {
            return Err("Array.slice requires Number or Null bounds".into());
        }
    }
    Ok(signature.result)
}

/// Check an instantiated API signature's arity, argument types, and witnesses.
fn check_arguments(
    signature: &api::InstantiatedMethod<'_>,
    arguments: &[api::Type],
    context: Option<api::ApiContext>,
) -> Result<(), String> {
    if arguments.len() > crate::MAX_PARAMETERS {
        return Err("native call exceeds 254-argument limit".into());
    }
    if arguments.len() < signature.parameters.len()
        || (signature.variadic.is_none() && signature.parameters.len() != arguments.len())
    {
        return Err("native call argument count does not match its API contract".into());
    }
    for (index, actual) in arguments.iter().enumerate() {
        if let Some(expected) = signature
            .parameters
            .get(index)
            .or(signature.variadic.as_ref())
        {
            structural_api(expected, actual, context)?;
        }
    }
    if !arguments.iter().enumerate().all(|(index, actual)| {
        signature
            .parameters
            .get(index)
            .or(signature.variadic.as_ref())
            .is_some_and(|expected| accepts_boundary(expected, actual))
    }) {
        return Err("native call argument types do not match its API contract".into());
    }
    Ok(())
}

/// Check the existing Array sorting profile for one possible element type.
fn sort_element_supported(element: &api::Type, arguments: &[api::Type]) -> bool {
    match crate::ArrayElement::from_api_type(element) {
        Some(crate::ArrayElement::Null) => false,
        Some(
            crate::ArrayElement::Numeric {
                nullable: false, ..
            }
            | crate::ArrayElement::Number
            | crate::ArrayElement::Long
            | crate::ArrayElement::Float
            | crate::ArrayElement::Double
            | crate::ArrayElement::Boolean
            | crate::ArrayElement::Char
            | crate::ArrayElement::String
            | crate::ArrayElement::LangObject
            | crate::ArrayElement::Interface(crate::InterfaceType::Comparable),
        ) => true,
        Some(element) if element.nullable_base().is_none() => accepts(
            &crate::InterfaceType::Comparator.api_type(false),
            &arguments[0],
        ),
        None if accepts(&api::Type::named("Toybox.Lang.Object"), element) => accepts(
            &crate::InterfaceType::Comparator.api_type(false),
            &arguments[0],
        ),
        _ => false,
    }
}

/// Check value profiles used to select supported Object.method receivers.
/// The caller separately excludes Null and union receivers.
fn method_receiver_profile(ty: &api::Type) -> bool {
    match ty {
        api::Type::TupleValue(tuple) => tuple.validate().is_ok(),
        api::Type::Null => true,
        api::Type::Named(name) => matches!(
            name.as_str(),
            "Toybox.Lang.Number"
                | "Toybox.Lang.Float"
                | "Toybox.Lang.Double"
                | "Toybox.Lang.Long"
                | "Toybox.Lang.String"
                | "Toybox.Lang.Boolean"
                | "Toybox.Lang.Symbol"
                | "Toybox.Lang.Char"
                | "Toybox.Lang.Dictionary"
                | "Toybox.Lang.Object"
                | "Toybox.Lang.Method"
                | "Toybox.Lang.WeakReference"
                | "Toybox.Lang.ByteArray"
        ),
        api::Type::Array(element) => crate::ArrayElement::from_api_type(element).is_some(),
        api::Type::Dictionary { .. } => crate::DictionaryType::from_api_type(ty).is_some(),
        api::Type::Union(types) => types.iter().all(method_receiver_profile),
        _ => false,
    }
}

/// Check the observed scalar Comparable implementations against the catalogue.
/// Their required members must be available by the interface introduction level,
/// so checking the receiving interface's API level also covers this witness.
fn scalar_comparable(actual: &api::Type) -> bool {
    let api::Type::Named(class) = actual else {
        return false;
    };
    if !matches!(
        class.as_str(),
        "Toybox.Lang.Number"
            | "Toybox.Lang.Long"
            | "Toybox.Lang.Float"
            | "Toybox.Lang.Double"
            | "Toybox.Lang.Boolean"
            | "Toybox.Lang.Char"
            | "Toybox.Lang.String"
    ) {
        return false;
    }
    let catalogue = api::core();
    let name = crate::InterfaceType::Comparable.name();
    let interface = catalogue.interface(name).unwrap();
    let level = catalogue.interface_level(name).unwrap();
    interface.methods.iter().all(|required| {
        catalogue
            .method(class, &required.name)
            .is_some_and(|actual| {
                actual.method.parameters == required.parameters
                    && actual.method.variadic == required.variadic
                    && actual.result == &required.result
                    && !actual.method.callback
                    && actual.required_api <= level
                    && required.since <= level
            })
    })
}

/// Accept a value boundary with matching or overlapping contracts.
/// Container parameters require equality or uncertain union overlap, including
/// nested containers; nominal inheritance alone does not match them. Other
/// alternatives remain possible runtime failures. Source alternatives retain
/// exact class identities; units resolve deferred structural witnesses.
/// Receiver profiles, arity, API, and metadata checks remain independent.
pub(crate) fn accepts_boundary(expected: &api::Type, actual: &api::Type) -> bool {
    if let (api::Type::SourceValue(expected), api::Type::SourceValue(actual)) = (expected, actual) {
        return expected.accepts_boundary(actual);
    }
    fn native_contract(ty: &api::Type) -> bool {
        match ty {
            api::Type::Named(_) | api::Type::Null | api::Type::Any => true,
            api::Type::Union(parts) => parts.iter().all(native_contract),
            api::Type::Array(element) => native_contract(element),
            api::Type::Dictionary { key, value } => native_contract(key) && native_contract(value),
            api::Type::SourceValue(contract) => contract.native().is_empty(),
            _ => false,
        }
    }
    accepts(expected, actual)
        || native_contract(expected)
            && native_contract(actual)
            && crate::type_check::compatibility(expected, actual) == crate::type_check::Match::Maybe
}

/// Check exact types, declared inheritance, verified scalar Comparable witnesses,
/// or union membership without implicit conversions. Every alternative of an
/// actual union must be accepted.
/// Containers may widen to their declared class/base; their parameters are invariant.
/// A positional tuple also satisfies Array<T> when every slot satisfies T;
/// an Array alone cannot establish a tuple's length or positional contracts.
/// This positional adaptation also applies inside nested Array element domains.
/// Any crosses value boundaries dynamically; Void remains separate from values.
pub(crate) fn accepts(expected: &api::Type, actual: &api::Type) -> bool {
    accepts_in(expected, actual, &crate::recursive::Comparison::default())
}
fn accepts_in(
    expected: &api::Type,
    actual: &api::Type,
    comparison: &crate::recursive::Comparison,
) -> bool {
    comparison
        .scoped(expected, actual, || {
            accepts_step(expected, actual, comparison)
        })
        .unwrap_or(true)
}
fn accepts_step(
    expected: &api::Type,
    actual: &api::Type,
    comparison: &crate::recursive::Comparison,
) -> bool {
    let accepts = |e: &api::Type, a: &api::Type| accepts_in(e, a, comparison);
    if matches!(expected, api::Type::Recursive(_)) || matches!(actual, api::Type::Recursive(_)) {
        return crate::tuples::matches_with(expected, actual, &accepts);
    }

    if let (api::Type::Dictionary { key, value }, api::Type::Options(fields)) = (expected, actual) {
        return accepts(key, &api::Type::named("Toybox.Lang.Symbol"))
            && fields.values().all(|ty| accepts(value, ty));
    }
    if let (api::Type::Options(expected), api::Type::Options(actual)) = (expected, actual) {
        return actual.iter().all(|(key, ty)| {
            expected
                .get(key)
                .is_none_or(|required| accepts(required, ty))
        });
    }
    if crate::structural::same_type(expected, actual)
        || (*expected == api::Type::Any || *actual == api::Type::Any)
            && *expected != api::Type::Void
            && *actual != api::Type::Void
    {
        return true;
    }
    if let api::Type::SourceValue(expected) = expected {
        return match actual {
            api::Type::SourceValue(actual) => expected.accepts_using(actual, &accepts, |_, ty| {
                matches!(
                    crate::ValueType::from_api_type(ty),
                    Some(crate::ValueType::Interface(_))
                )
            }),
            _ => accepts(&expected.native_type(), actual),
        };
    }
    if let api::Type::SourceValue(actual) = actual {
        return accepts(expected, &actual.api_type());
    }
    if matches!(expected, api::Type::Named(name) if name == crate::InterfaceType::Comparable.name())
        && scalar_comparable(actual)
    {
        return true;
    }
    if let api::Type::Callback(expected_signature) = expected {
        return match actual {
            api::Type::Callback(actual_signature) => expected_signature.accepts(actual_signature),
            api::Type::Named(name) if name == "Toybox.Lang.Method" => true,
            api::Type::Union(parts) => parts.iter().all(|ty| accepts(expected, ty)),
            _ => false,
        };
    }
    if let api::Type::Callback(_) = actual {
        if let api::Type::Structural(shape) = expected {
            return shape.accepts(actual);
        }
        return matches!(expected,api::Type::Named(name) if name=="Toybox.Lang.Method" || name=="Toybox.Lang.Object")
            || matches!(expected,api::Type::Union(parts) if parts.iter().any(|ty|accepts(ty,actual)));
    }
    if let api::Type::Union(types) = actual {
        return types.iter().all(|ty| accepts(expected, ty));
    }
    if let api::Type::Union(types) = expected {
        return types.iter().any(|ty| accepts(ty, actual));
    }
    if let (api::Type::Array(expected), api::Type::Array(actual)) = (expected, actual) {
        return crate::tuples::array_elements_match_with(expected, actual, &accepts);
    }
    if let api::Type::TupleValue(expected) = expected {
        return match actual {
            api::Type::TupleValue(actual) => {
                expected.slots().len() == actual.slots().len()
                    && expected.slots().iter().zip(actual.slots()).all(|(e, a)| {
                        crate::tuples::matches_with(
                            &crate::tuples::exact_type(e),
                            &crate::tuples::exact_type(a),
                            &accepts,
                        )
                    })
            }
            api::Type::Tuple(actual) => {
                expected.slots().len() == actual.len()
                    && expected
                        .slots()
                        .iter()
                        .zip(actual)
                        .all(|(e, a)| e.source_contract().is_none() && accepts(&e.api_type(), a))
            }
            _ => false,
        };
    }
    if let api::Type::TupleValue(actual) = actual {
        if let api::Type::Array(element) = expected {
            return actual.slots().iter().all(|slot| {
                let ty = slot
                    .source_contract()
                    .map(|c| api::Type::SourceValue(Box::new(c)))
                    .unwrap_or_else(|| slot.api_type());
                accepts(element, &ty)
            });
        }
        return accepts(
            expected,
            &api::Type::Tuple(actual.slots().iter().map(ReturnType::api_type).collect()),
        );
    }
    if let api::Type::Structural(shape) = expected {
        return shape.accepts(actual);
    }
    if let api::Type::Structural(_) = actual {
        if let api::Type::Named(name) = expected {
            if let Some(required) = crate::StructuralInterface::from_api(name) {
                return required.accepts(actual);
            }
        }
        return *expected == api::Type::named("Toybox.Lang.Object");
    }
    if let (api::Type::Tuple(expected), api::Type::Tuple(actual)) = (expected, actual) {
        return expected.len() == actual.len()
            && expected
                .iter()
                .zip(actual)
                .all(|(expected, actual)| accepts(expected, actual));
    }
    if let (api::Type::Array(element), api::Type::Tuple(slots)) = (expected, actual) {
        return slots.iter().all(|slot| accepts(element, slot));
    }
    if let api::Type::Named(expected) = expected {
        let actual = match actual {
            api::Type::Named(actual) => actual.as_str(),
            api::Type::Array(_) | api::Type::Tuple(_) => "Toybox.Lang.Array",
            api::Type::Dictionary { .. } | api::Type::Options(_) => "Toybox.Lang.Dictionary",
            _ => return false,
        };
        if expected == "Toybox.Lang.Object" && crate::InterfaceType::from_name(actual).is_some() {
            return true;
        }
        // Recorded enums use Number values at runtime, while their nominal
        // contracts still reject implicit substitution at scalar boundaries.
        if expected == "Toybox.Lang.Object" && api::core().enum_level(actual).is_some() {
            return true;
        }
        let mut class = api::core().class(actual);
        while let Some(current) = class {
            if current.name == *expected {
                return true;
            }
            class = current
                .superclass
                .as_deref()
                .and_then(|name| api::core().class(name));
        }
    }
    false
}

/// Check API introductions used to prove a native structural implementation.
/// A lexical method name alone has no API level; a known native implementation
/// must provide that member on both selected levels. Type validity is separate.
pub(crate) fn structural_api(
    expected: &api::Type,
    actual: &api::Type,
    context: Option<api::ApiContext>,
) -> Result<(), String> {
    structural_api_in(
        expected,
        actual,
        context,
        &crate::recursive::Comparison::default(),
    )
}
fn structural_api_in(
    expected: &api::Type,
    actual: &api::Type,
    context: Option<api::ApiContext>,
    comparison: &crate::recursive::Comparison,
) -> Result<(), String> {
    comparison
        .scoped(expected, actual, || {
            structural_api_step(expected, actual, context, comparison)
        })
        .unwrap_or(Ok(()))
}
fn structural_api_step(
    expected: &api::Type,
    actual: &api::Type,
    context: Option<api::ApiContext>,
    comparison: &crate::recursive::Comparison,
) -> Result<(), String> {
    let structural_api =
        |e: &api::Type, a: &api::Type, context| structural_api_in(e, a, context, comparison);
    if crate::structural::same_type(expected, actual) {
        if let Some(context) = context {
            context.check_type(api::core(), expected)?;
            context.check_type(api::core(), actual)?;
        }
        return Ok(());
    }
    if let Some(body) = crate::recursive::unfold(expected) {
        return structural_api(&body, actual, context);
    }
    if let Some(body) = crate::recursive::unfold(actual) {
        return structural_api(expected, &body, context);
    }

    let Some(context) = context else {
        return Ok(());
    };
    if (!crate::structural::contains(expected)
        && !crate::tuples::contains(expected)
        && !crate::tuples::contains(actual))
        || *actual == api::Type::Any
        || matches!(actual, api::Type::SourceValue(_))
        || !accepts(expected, actual)
    {
        return Ok(());
    }
    if let api::Type::Union(parts) = actual {
        return parts
            .iter()
            .try_for_each(|ty| structural_api(expected, ty, Some(context)));
    }
    if let api::Type::Union(parts) = expected {
        let mut last = None;
        for ty in parts.iter().filter(|ty| accepts(ty, actual)) {
            match structural_api(ty, actual, Some(context)) {
                Ok(()) => return Ok(()),
                Err(error) => last = Some(error),
            }
        }
        return Err(last.unwrap_or_else(|| "native structural witness is unavailable".into()));
    }
    if let (api::Type::Options(expected), api::Type::Options(actual)) = (expected, actual) {
        return actual
            .iter()
            .filter_map(|(key, ty)| expected.get(key).map(|required| (required, ty)))
            .try_for_each(|(e, a)| structural_api(e, a, Some(context)));
    }
    if let api::Type::TupleValue(actual) = actual {
        let expected_slots = match expected {
            api::Type::TupleValue(tuple) => tuple
                .slots()
                .iter()
                .map(crate::tuples::exact_type)
                .collect::<Vec<_>>(),
            api::Type::Array(element) => vec![(**element).clone(); actual.slots().len()],
            _ => return Ok(()),
        };
        return expected_slots
            .iter()
            .zip(actual.slots())
            .try_for_each(|(expected, actual)| {
                structural_api(expected, &crate::tuples::exact_type(actual), Some(context))
            });
    }
    if let (api::Type::Array(expected), api::Type::Array(actual)) = (expected, actual) {
        return structural_api(expected, actual, Some(context));
    }
    if let api::Type::Structural(shape) = expected {
        if matches!(actual, api::Type::Structural(_)) {
            return Ok(());
        }
        context.check_type(api::core(), actual)?;
        let actual_native = if matches!(actual, api::Type::Callback(_)) {
            api::Type::named("Toybox.Lang.Method")
        } else {
            actual.clone()
        };
        for name in shape.methods.keys() {
            let required = api::core()
                .method_for_type(&actual_native, name)
                .map(|method| method.declaration.required_api)
                .or_else(|| {
                    if let api::Type::Named(path) = actual {
                        api::core()
                            .interface(path)?
                            .methods
                            .iter()
                            .find(|method| method.name == *name)
                            .map(|method| method.since)
                    } else {
                        None
                    }
                })
                .ok_or("native structural member is not declared")?;
            context.require(&format!("{actual:?}.{name}"), required)?;
        }
    }
    Ok(())
}

/// Resolve a module function's parameter declarations for literal construction.
/// Availability and executable support are checked when validating the call.
pub(crate) fn module_parameters(module: &str, member: &str) -> Option<Vec<api::Type>> {
    api::core()
        .module(module)?
        .functions
        .iter()
        .find(|function| function.name == member)
        .map(|function| function.parameters.clone())
}

/// Check a value-producing native module call and return its API result type.
/// Argument assignability preserves typed containers at erased API boundaries.
pub(crate) fn module_value(
    module: &str,
    member: &str,
    arguments: &[api::Type],
    context: Option<api::ApiContext>,
) -> Result<api::Type, String> {
    let method = module_contract(module, member, arguments, context)?;
    if !matches!((module, member), ("Toybox.Lang", "format")) {
        return Err("native module member has no supported value lowering".into());
    }
    Ok(method.result.clone())
}

/// Resolve a module statement contract, then require supported Void lowering.
/// Declarations and introduction levels are checked before executable support.
pub(crate) fn module_statement(
    module: &str,
    member: &str,
    arguments: &[api::Type],
    context: Option<api::ApiContext>,
) -> Result<(), String> {
    let method = module_contract(module, member, arguments, context)?;
    if method.result != api::Type::Void
        || !matches!(
            (module, member),
            ("Toybox.System", "exit") | ("Toybox.WatchUi", "requestUpdate")
        )
    {
        return Err("native module member has no supported statement lowering".into());
    }
    Ok(())
}

/// Resolve a canonical module function and check availability, arity, and types.
fn module_contract(
    module: &str,
    member: &str,
    arguments: &[api::Type],
    context: Option<api::ApiContext>,
) -> Result<&'static api::Method, String> {
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
    if !method
        .parameters
        .iter()
        .zip(arguments)
        .all(|(expected, actual)| accepts_boundary(expected, actual))
    {
        return Err("native call argument types do not match its API contract".into());
    }
    Ok(method)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuple_assignability_checks_each_position_and_array_element() {
        use api::Type;
        let number = Type::named("Toybox.Lang.Number");
        let string = Type::named("Toybox.Lang.String");
        let pair = Type::Tuple(vec![number.clone(), string.clone()]);
        let numeric = api::Type::numeric();
        let array = |element| Type::Array(Box::new(element));

        assert!(accepts(
            &Type::Tuple(vec![numeric.clone(), string.clone()]),
            &pair
        ));
        assert!(!accepts(
            &pair,
            &Type::Tuple(vec![numeric.clone(), string.clone()])
        ));
        assert!(!accepts(&Type::Tuple(vec![number.clone()]), &pair));
        assert!(!accepts(
            &Type::Tuple(vec![string.clone(), number.clone()]),
            &pair
        ));

        let elements = crate::unions::normalize(vec![number.clone(), string]);
        assert!(accepts(&array(elements.clone()), &pair));
        assert!(accepts(&array(Type::named("Toybox.Lang.Object")), &pair));
        assert!(!accepts(&array(number.clone()), &pair));
        assert!(!accepts(&pair, &array(elements)));
        assert!(!accepts(&array(numeric.clone()), &array(number.clone())));
        assert!(!accepts(&pair, &Type::Null));

        let maybe_number = Type::Tuple(vec![number.clone().with_null()]);
        assert!(accepts(&array(number.clone().with_null()), &maybe_number));
        assert!(!accepts(&array(number.clone()), &maybe_number));
        assert!(accepts(
            &array(numeric),
            &Type::Union(vec![
                Type::Tuple(vec![number]),
                Type::Tuple(vec![Type::named("Toybox.Lang.Float")]),
            ])
        ));
        assert!(!accepts(
            &array(Type::Any),
            &Type::Union(vec![pair, Type::Null])
        ));
    }

    #[test]
    fn dictionary_assignability_is_invariant_and_erasure_is_one_way() {
        let string = api::Type::named("Toybox.Lang.String");
        let number = api::Type::named("Toybox.Lang.Number");
        let object = api::Type::named("Toybox.Lang.Object");
        let exact = api::Type::dictionary(string.clone(), number.clone());
        assert!(accepts(&exact, &exact));
        for other in [
            api::Type::dictionary(object.clone(), number.clone()),
            api::Type::dictionary(string.clone(), object.clone()),
            api::Type::dictionary(string, api::Type::Union(vec![number, api::Type::Null])),
        ] {
            assert!(!accepts(&other, &exact));
            assert!(!accepts(&exact, &other));
        }
        let opaque = api::Type::named("Toybox.Lang.Dictionary");
        assert!(accepts(&opaque, &exact));
        assert!(accepts(&object, &exact));
        assert!(!accepts(&exact, &opaque));
        assert!(!accepts(&exact, &api::Type::Null));
        assert!(!accepts(&api::Type::Array(Box::new(object)), &exact));

        assert_eq!(value_parameters(&exact, "put").unwrap().len(), 2);
        assert_eq!(
            value_call(&exact, "size", &[], None).unwrap(),
            api::Type::named("Toybox.Lang.Number")
        );
        assert_eq!(
            value_call(
                &exact,
                "get",
                &[api::Type::named("Toybox.Lang.String")],
                None
            )
            .unwrap(),
            api::Type::Union(vec![
                api::Type::named("Toybox.Lang.Number"),
                api::Type::Null
            ])
        );
        assert!(
            value_call(
                &opaque,
                "get",
                &[api::Type::named("Toybox.Lang.String")],
                None
            )
            .is_err()
        );
    }

    #[test]
    fn opaque_object_weak_result_stays_unsupported() {
        let receiver = api::Type::named("Toybox.Lang.Object");
        assert!(
            value_call(&receiver, "weak", &[], None)
                .unwrap_err()
                .contains("no supported lowering")
        );
    }

    #[test]
    fn assignability_rejects_unknown_classes_and_unrelated_or_nullable_types() {
        let chars = api::Type::Array(Box::new(api::Type::named("Toybox.Lang.Char")));
        let objects = api::Type::Array(Box::new(api::Type::named("Toybox.Lang.Object")));
        assert!(accepts(&chars, &chars));
        assert!(accepts(&api::Type::named("Toybox.Lang.Object"), &chars));
        assert!(!accepts(&objects, &chars));
        assert!(!accepts(&chars, &objects));
        let object = api::Type::named("Toybox.Lang.Object");
        let number = api::Type::named("Toybox.Lang.Number");
        let boolean = api::Type::named("Toybox.Lang.Boolean");
        let nullable = api::Type::Union(vec![number.clone(), api::Type::Null]);
        assert!(accepts(&object, &number));
        assert!(accepts(&object, &boolean));
        assert!(!accepts(&object, &api::Type::named("Unknown")));
        assert!(!accepts(&object, &nullable));
        assert!(!accepts(&number, &boolean));
        assert!(!accepts(&number, &nullable));
        assert!(accepts(
            &api::Type::Union(vec![object, api::Type::Null]),
            &nullable
        ));
    }
}
