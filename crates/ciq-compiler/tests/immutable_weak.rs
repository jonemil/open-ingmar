use ciq_compiler::{
    api::{ApiContext, ApiLevel, Type},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources_with_api, compile_with_api, parse_ir, parse_ir_with_api,
    parse_unit_ir_with_api,
};

fn context(level: ApiLevel) -> ApiContext {
    ApiContext::new(level, level).unwrap()
}

#[test]
fn immutable_weak_preserves_result_types_and_complete_pipelines() {
    let context = context(ApiLevel(1, 2, 0));
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-immutable-weak/app.mc"),
            include_str!("../../../examples/compiler-immutable-weak/values.mc"),
        ],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [27; 16],
        entry: "start".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    for source in [
        "function f() { return 7.weak() + 1; }",
        "function f() as Toybox.Lang.String { return \"a\".weak().toUpper(); }",
        "function f() as Toybox.Lang.Boolean { return true.weak(); }",
        "function f(n as Toybox.Lang.Number or Null) { if (n == null) { return 0; } return n.weak(); }",
        "function f(s as Toybox.Lang.String or Null) as Toybox.Lang.String { if (s == null) { return \"\"; } return s.weak(); }",
    ] {
        let f = compile(source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        let symbols = ciq_assembly::Program::parse(
            ".symbol weak \"weak\" id=0x00800418\n.symbol upper \"toUpper\" id=0x008000bf",
        )
        .unwrap();
        assert_eq!(
            ciq_compiler::symbol_bindings(&f.lower(), &symbols).unwrap()["__ciq_native_weak"],
            0x00800418
        );
    }
}

#[test]
fn weak_contracts_reject_nullable_receivers_wrong_arity_and_consumers() {
    for source in [
        "function f() { return 1.weak(2); }",
        "function f() { return null.weak(); }",
        "function f(n as Toybox.Lang.Number or Null) { return n.weak(); }",
        "function f(s as Toybox.Lang.String or Null) as Toybox.Lang.String { return s.weak(); }",
        "function f() { return true.weak() + 1; }",
        "function f() { return \"a\".weak(); }",
        "function f() as Toybox.Lang.Boolean { return 1.weak(); }",
        "function f() { if (1.weak()) { return 1; } return 0; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let f = compile("function f() { return 1.weak() + 1; }").unwrap();
    let text = f.to_ir_text();
    assert!(text.contains(".integer %0 1"));
    assert!(parse_ir(&text.replace(".integer %0 1", ".boolean %0 true")).is_err());
    let f = compile("function f(n as Toybox.Lang.Number or Null) { if (n == null) { return 0; } return n.weak(); }").unwrap();
    let mut operations = f.operations().to_vec();
    for op in &mut operations {
        if let ciq_compiler::Operation::Branch {
            on_true, on_false, ..
        } = &mut op.operation
        {
            std::mem::swap(on_true, on_false);
        }
    }
    assert!(
        ciq_compiler::Function::with_signature(
            "bad".into(),
            vec![ciq_compiler::ParameterType::NullableInteger],
            ciq_compiler::ReturnType::Integer,
            operations
        )
        .is_err()
    );
}

#[test]
fn weak_declared_and_effective_results_remain_distinct_and_versioned() {
    let catalogue = ciq_compiler::api::core();
    for name in ["Number", "String", "Boolean"] {
        let class = format!("Toybox.Lang.{name}");
        let resolved = catalogue.method(&class, "weak").unwrap();
        assert_eq!(resolved.declaring_class.name, "Toybox.Lang.Object");
        assert_eq!(
            resolved.method.result,
            Type::named("Toybox.Lang.WeakReference")
        );
        assert_eq!(*resolved.result, Type::named(class));
    }
    for name in ["Object", "Dictionary", "Char"] {
        assert_eq!(
            *catalogue
                .method(&format!("Toybox.Lang.{name}"), "weak")
                .unwrap()
                .result,
            Type::named("Toybox.Lang.WeakReference")
        );
    }
    for source in [
        "function f() { return 1.weak(); }",
        "function f() as Toybox.Lang.String { return \"a\".weak(); }",
        "function f() as Toybox.Lang.Boolean { return true.weak(); }",
    ] {
        let text = compile(source).unwrap().to_ir_text();
        for (target, minimum, expected) in [
            (
                ApiLevel(1, 1, 9),
                ApiLevel(1, 0, 0),
                "unavailable on target",
            ),
            (
                ApiLevel(1, 2, 0),
                ApiLevel(1, 1, 9),
                "application minimum API",
            ),
        ] {
            let context = ApiContext::new(target, minimum).unwrap();
            let error = compile_with_api(source, context).unwrap_err();
            assert!(error.message.contains(expected));
            assert_eq!(
                error.message,
                parse_ir_with_api(&text, context).unwrap_err().message
            );
        }
        let context = context(ApiLevel(1, 2, 0));
        assert_eq!(
            compile_with_api(source, context).unwrap(),
            parse_ir_with_api(&text, context).unwrap()
        );
    }
}
