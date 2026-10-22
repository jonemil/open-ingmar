use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources, parse_function_ir, parse_unit_ir,
};

#[test]
fn substring_preserves_complete_app_and_text_pipelines() {
    let unit = compile_sources(&[
        include_str!("../../../examples/compiler-substring/app.mc"),
        include_str!("../../../examples/compiler-substring/slices.mc"),
    ])
    .unwrap();
    unit.validate_api(ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(3, 3, 2)).unwrap())
        .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let app = ConsoleApplication {
        id: [22; 16],
        entry: "start".into(),
    };
    assert_eq!(
        build_console_application(&unit, &app).unwrap(),
        build_console_application(&resumed, &app).unwrap()
    );
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    let symbols =
        ciq_assembly::Program::parse(".symbol slice \"substring\" id=0x008000bd").unwrap();
    let function = compile_function("function f() { var s = \"hello\".substring(0, 2); if (s == null) { return 0; } return s.length(); }").unwrap();
    let bindings = ciq_compiler::symbol_bindings(
        &function.lower(),
        &ciq_assembly::Program::parse(&format!(
            "{}\n.symbol length \"length\" id=0x008000bb",
            symbols.to_text()
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(bindings["__ciq_native_substring"], 0x008000bd);
    assert!(function.lower().to_text().contains("invokem 0x03"));
}

#[test]
fn string_proofs_survive_ir_and_reject_unsafe_consumers() {
    for source in [
        "function f() as Toybox.Lang.String { var s = \"hello\".substring(0, 2); if (s == null) { return \"fallback\"; } return s; }",
        "function f() { var s = \"hello\".substring(0, 2); if (null == s) { return 0; } if (s == null) { return 0; } return s.length(); }",
        "function f() { var s = \"hello\".substring(0, 2); while (s == null) { s = \"hello\".substring(1, 2); } return s.length(); }",
    ] {
        let f = compile_function(source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    for source in [
        "function f() { System.println(\"hello\".substring(0, 2)); }",
        "function f() as Toybox.Lang.String { return \"hello\".substring(0, 2); }",
        "function f() { return \"hello\".substring(0, 2).length(); }",
        "function f() { var s = \"hello\".substring(0, 2); if (s == null) { return s.length(); } return 0; }",
        "function f() { var s = \"hello\".substring(0, 2); if (s == null) { return 0; } s = null; return s.length(); }",
        "function f() { var s = \"hello\".substring(0, 2); if (s == null) { return 0; } s = null; return s.length(); }",
        "function f() { var s = \"hello\".substring(0, 2); if (s == null) { System.println(0); } else { System.println(s); } return s.length(); }",
        "function f() { var s = \"hello\".substring(0, 2); while (s == null) { break; } return s.length(); }",
        "function f() { var s = \"hello\".substring(0, 2); var other = \"hello\".substring(0, 1); if (s == null) { return 0; } return other.length(); }",
        "function f() { var s = \"hello\".substring(0, 2); if (s == null) { return 0; } s = true; return s.length(); }",
        "function f() { var s = \"hello\".substring(true, 2); return 0; }",
        "function f() { var s = \"hello\".substring(\"0\", 2); return 0; }",
        "function f() { var s = \"hello\".substring(0); return 0; }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    let f = compile_function("function f() { var s = \"hello\".substring(0, 2); if (s == null) { return 0; } return s.length(); }").unwrap();
    let mut operations = f.operations().to_vec();
    for op in &mut operations {
        if let ciq_compiler::Operation::Branch {
            on_true, on_false, ..
        } = &mut op.operation
        {
            std::mem::swap(on_true, on_false);
        }
    }
    assert!(ciq_compiler::Function::new("unsafe".into(), operations).is_err());
}

#[test]
fn nullable_index_arguments_require_their_api_revision() {
    let numeric =
        compile_function("function f() { var s = \"hello\".substring(0, 2); return 0; }").unwrap();
    numeric
        .validate_api(ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap())
        .unwrap();
    for source in [
        "function f() { var s = \"hello\".substring(null, 2); return 0; }",
        "function f() { var s = \"hello\".substring(0, null); return 0; }",
        "function f() { var n = \"bad\".toNumber(); var s = \"hello\".substring(n, 2); return 0; }",
    ] {
        let f = compile_function(source).unwrap();
        for (target, minimum) in [
            (ApiLevel(3, 3, 1), ApiLevel(1, 0, 0)),
            (ApiLevel(5, 2, 0), ApiLevel(3, 3, 1)),
        ] {
            assert!(
                f.validate_api(ApiContext::new(target, minimum).unwrap())
                    .is_err()
            );
        }
        let context = ApiContext::new(ApiLevel(3, 3, 2), ApiLevel(3, 3, 2)).unwrap();
        f.validate_api(context).unwrap();
        parse_function_ir(&f.to_ir_text())
            .unwrap()
            .validate_api(context)
            .unwrap();
    }
}
