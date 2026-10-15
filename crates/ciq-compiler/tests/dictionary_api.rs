use ciq_compiler::api::{ApiContext, ApiLevel, Type, core};

fn named(name: &str) -> Type {
    Type::named(format!("Toybox.Lang.{name}"))
}

#[test]
fn dictionary_signatures_preserve_keys_values_and_nullable_lookup() {
    let receiver = Type::dictionary(named("String"), named("Number"));
    let get = core().method_for_type(&receiver, "get").unwrap();
    assert_eq!(get.parameters, [named("String")]);
    assert_eq!(get.result, Type::Union(vec![named("Number"), Type::Null]));
    assert_eq!(
        get.declaration.declaring_class.name,
        "Toybox.Lang.Dictionary"
    );
    assert_eq!(get.declaration.required_api, ApiLevel(1, 0, 0));
    assert!(matches!(
        get.declaration.method.parameters[0],
        Type::Parameter(_)
    ));

    let put = core().method_for_type(&receiver, "put").unwrap();
    assert_eq!(put.parameters, [named("String"), named("Number")]);
    assert_eq!(put.result, Type::Void);
    for (member, result) in [("hasKey", named("Boolean")), ("remove", Type::Void)] {
        let signature = core().method_for_type(&receiver, member).unwrap();
        assert_eq!(signature.parameters, [named("String")]);
        assert_eq!(signature.result, result);
    }
    for (member, element) in [("keys", named("String")), ("values", named("Number"))] {
        let signature = core().method_for_type(&receiver, member).unwrap();
        assert!(signature.parameters.is_empty());
        assert_eq!(signature.result, Type::Array(Box::new(element)));
    }

    let nullable = Type::Union(vec![named("Number"), Type::Null]);
    let receiver = Type::dictionary(named("String"), nullable.clone());
    assert_eq!(
        core().method_for_type(&receiver, "get").unwrap().result,
        nullable
    );
    assert_eq!(
        core().method_for_type(&receiver, "put").unwrap().parameters[1],
        nullable
    );
    assert_eq!(
        core().method_for_type(&receiver, "values").unwrap().result,
        Type::Array(Box::new(nullable))
    );
    let all_null = Type::dictionary(named("String"), Type::Null);
    assert_eq!(
        core().method_for_type(&all_null, "get").unwrap().result,
        Type::Null
    );
}

#[test]
fn opaque_receivers_do_not_invent_generic_contracts_or_enable_lowering() {
    let receiver = named("Dictionary");
    assert!(core().method_for_type(&receiver, "size").is_some());
    assert!(core().method_for_type(&receiver, "get").is_none());
    assert!(core().method_for_type(&receiver, "put").is_none());
    assert!(core().method_for_type(&receiver, "keys").is_none());
    assert!(core().method_for_type(&Type::Null, "size").is_none());
    assert!(
        core()
            .method_for_type(
                &Type::dictionary(named("String"), named("Number")),
                "missing"
            )
            .is_none()
    );
    for source in [
        "function f(d as Toybox.Lang.Dictionary) { return d.get(\"one\"); }",
        "function f(d as Toybox.Lang.Dictionary) { d.put(\"one\", 1); return 0; }",
        "function f(d as Toybox.Lang.Dictionary) as Toybox.Lang.String { return d.toString(); }",
    ] {
        assert!(ciq_compiler::compile(source).is_err(), "accepted {source}");
    }
}

#[test]
fn dictionary_override_and_parameter_availability_are_checked_separately() {
    let receiver = Type::dictionary(named("String"), named("Number"));
    let method = core().method_for_type(&receiver, "toString").unwrap();
    assert_eq!(method.result, named("String"));
    assert_eq!(
        method.declaration.declaring_class.name,
        "Toybox.Lang.Dictionary"
    );
    assert!(!method.declaration.available_at(ApiLevel(1, 0, 0)));
    assert!(method.declaration.available_at(ApiLevel(1, 0, 1)));
    assert_eq!(
        core()
            .method_for_type(&receiver, "hashCode")
            .unwrap()
            .declaration
            .declaring_class
            .name,
        "Toybox.Lang.Object"
    );

    let early = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    early.check_type(core(), &receiver).unwrap();
    for receiver in [
        Type::dictionary(named("Char"), named("Number")),
        Type::dictionary(named("String"), Type::Array(Box::new(named("Char")))),
        Type::dictionary(
            named("String"),
            Type::dictionary(named("String"), named("Char")),
        ),
    ] {
        assert!(
            early
                .check_type(core(), &receiver)
                .unwrap_err()
                .contains("1.3.0")
        );
        let later = ApiContext::new(ApiLevel(1, 3, 0), ApiLevel(1, 3, 0)).unwrap();
        later.check_type(core(), &receiver).unwrap();
        let lower_minimum = ApiContext::new(ApiLevel(1, 3, 0), ApiLevel(1, 0, 0)).unwrap();
        assert!(lower_minimum.check_type(core(), &receiver).is_err());
    }
    for receiver in [
        Type::dictionary(Type::named("Unknown"), named("Number")),
        Type::dictionary(named("String"), Type::Parameter("Value".into())),
    ] {
        assert!(early.check_type(core(), &receiver).is_err());
    }
}

#[test]
fn common_instantiation_preserves_array_and_exact_receiver_contracts() {
    let receiver = Type::Array(Box::new(named("Number")));
    let add = core().method_for_type(&receiver, "add").unwrap();
    assert_eq!(add.parameters, [named("Number")]);
    assert_eq!(add.result, receiver);
    let slice = core().method_for_type(&receiver, "slice").unwrap();
    assert_eq!(
        slice.parameters,
        vec![Type::Union(vec![named("Number"), Type::Null]); 2]
    );
    assert_eq!(slice.result, receiver);
    assert_eq!(
        core()
            .method_for_type(&named("Number"), "weak")
            .unwrap()
            .result,
        named("Number")
    );
    assert_eq!(
        core()
            .method_for_type(&named("Dictionary"), "weak")
            .unwrap()
            .result,
        named("WeakReference")
    );
}
