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
        &self.result == result || self.alternative.as_ref() == Some(result)
    }
}

/// Resolve the bounded font constant with its enum type and published value.
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
        ReturnType::Tuple(crate::TupleShape::OneView),
    ]
    .into_iter()
    .find(|candidate| accepts(&method.result, &candidate.api_type()))?;
    let alternative = accepts(&method.result, &crate::TupleShape::ViewDelegate.api_type())
        .then_some(ReturnType::Tuple(crate::TupleShape::ViewDelegate));
    Some(CallbackSignature {
        parameters,
        result,
        alternative,
    })
}

/// Resolve a value-call contract through the catalogue, then require its bounded
/// executable profile. Return the effective receiver-specific result; declaration
/// availability, arguments, and executable support must all pass first.
pub(crate) fn value_call(
    receiver: &api::Type,
    member: &str,
    arguments: &[api::Type],
    context: Option<api::ApiContext>,
) -> Result<api::Type, String> {
    let class = match receiver {
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
    if signature.parameters.len() != arguments.len() {
        return Err("native call argument count does not match its API contract".into());
    }
    if !signature
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
        (class, resolved.declaring_class.name.as_str(), member),
        (
            "Toybox.Lang.Number" | "Toybox.Lang.String" | "Toybox.Lang.Boolean",
            "Toybox.Lang.Object",
            "weak"
        ) | (
            "Toybox.Lang.Number" | "Toybox.Lang.Boolean" | "Toybox.Lang.Char",
            "Toybox.Lang.Object",
            "toString" | "equals" | "hashCode"
        ) | ("Toybox.Lang.Boolean", "Toybox.Lang.Boolean", "compareTo")
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
            )
            | (
                "Toybox.Lang.Number",
                "Toybox.Lang.Number",
                "format" | "abs" | "toNumber" | "toChar" | "compareTo"
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
                    | "toCharArray"
                    | "toUtf8Array"
            )
    );
    let supported = supported
        || (class == "Toybox.Graphics.Dc" && matches!(member, "clear" | "setColor" | "drawText"));
    if !supported {
        return Err("native member has no supported lowering for this receiver".into());
    }
    if matches!(
        class,
        "Toybox.Lang.Number" | "Toybox.Lang.Boolean" | "Toybox.Lang.Char"
    ) && member == "compareTo"
        && !matches!(arguments, [api::Type::Named(name)] if matches!(name.as_str(), "Toybox.Lang.Number" | "Toybox.Lang.Boolean" | "Toybox.Lang.Char"))
    {
        return Err(format!(
            "{class}.compareTo lowering currently requires Number, Boolean, or Char"
        ));
    }
    if class == "Toybox.Lang.String"
        && member == "compareTo"
        && arguments != [api::Type::named("Toybox.Lang.String")]
    {
        return Err("String.compareTo lowering currently requires String".into());
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
    if member == "equals" && !scalar_or_null(&arguments[0]) {
        return Err(format!(
            "{class}.equals lowering currently requires scalar or nullable scalar arguments"
        ));
    }
    Ok(signature.result)
}

fn scalar_or_null(ty: &api::Type) -> bool {
    match ty {
        api::Type::Null => true,
        api::Type::Named(name) => matches!(
            name.as_str(),
            "Toybox.Lang.Number"
                | "Toybox.Lang.String"
                | "Toybox.Lang.Boolean"
                | "Toybox.Lang.Char"
        ),
        api::Type::Union(types) => types.iter().all(scalar_or_null),
        _ => false,
    }
}

/// Check exact types, declared inheritance, or union membership without implicit
/// conversions. Every alternative of an actual union must be accepted.
/// Containers may widen to their declared class/base; their parameters are invariant.
pub(crate) fn accepts(expected: &api::Type, actual: &api::Type) -> bool {
    if expected == actual {
        return true;
    }
    if let api::Type::Union(types) = actual {
        return types.iter().all(|ty| accepts(expected, ty));
    }
    if let api::Type::Union(types) = expected {
        return types.iter().any(|ty| accepts(ty, actual));
    }
    if let (api::Type::Tuple(expected), api::Type::Tuple(actual)) = (expected, actual) {
        return expected.len() == actual.len()
            && expected
                .iter()
                .zip(actual)
                .all(|(expected, actual)| accepts(expected, actual));
    }
    if let api::Type::Named(expected) = expected {
        let actual = match actual {
            api::Type::Named(actual) => actual.as_str(),
            api::Type::Array(_) | api::Type::Tuple(_) => "Toybox.Lang.Array",
            api::Type::Dictionary { .. } => "Toybox.Lang.Dictionary",
            _ => return false,
        };
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn weak_reference_declaration_does_not_enable_object_lowering() {
        for class in ["Object", "Dictionary", "Char", "WeakReference"] {
            let receiver = api::Type::named(format!("Toybox.Lang.{class}"));
            assert!(
                value_call(&receiver, "weak", &[], None)
                    .unwrap_err()
                    .contains("no supported lowering")
            );
        }
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
