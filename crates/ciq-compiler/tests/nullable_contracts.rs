use ciq_compiler::{
    Function, ParameterType, ReturnType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, parse_ir, parse_unit_ir,
};

#[test]
fn nullable_user_contracts_preserve_complete_app_and_text() {
    let unit = compile_sources(&[
        include_str!("../../../examples/compiler-nullable-contracts/app.mc"),
        include_str!("../../../examples/compiler-nullable-contracts/values.mc"),
    ])
    .unwrap();
    let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap();
    unit.validate_api(context).unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    resumed.validate_api(context).unwrap();
    assert_eq!(unit, resumed);
    let app = ConsoleApplication {
        id: [23; 16],
        entry: "NullableApp.onStart".into(),
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
    let text = unit.to_ir_text();
    assert!(text.contains("parameter-types=nullable-integer"));
    assert!(text.contains("parameter-types=nullable-string"));
    assert!(text.contains("result=nullable-integer"));
    assert!(text.contains("result=nullable-string"));
    // Calls cannot hide the callee's union behind a scalar result annotation.
    let edited = text.replace(
        "passString result=nullable-string",
        "passString result=string",
    );
    assert_ne!(text, edited);
    assert!(parse_unit_ir(&edited).is_err());
}

#[test]
fn explicit_nullable_results_accept_only_the_declared_alternatives() {
    for (source, result) in [
        (
            "function f() as Toybox.Lang.Number or Null { return null; }",
            ReturnType::NullableInteger,
        ),
        (
            "function f() as Toybox.Lang.Number or Null { return 7; }",
            ReturnType::NullableInteger,
        ),
        (
            "function f() as Toybox.Lang.String or Null { return null; }",
            ReturnType::NullableString,
        ),
        (
            "function f() as Toybox.Lang.String or Null { return \"hi\"; }",
            ReturnType::NullableString,
        ),
        (
            "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Number or Null { if (flag) { return null; } return 7; }",
            ReturnType::NullableInteger,
        ),
    ] {
        let f = compile(source).unwrap();
        assert_eq!(f.return_type(), result);
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        f.validate_api(ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap())
            .unwrap();
        assert!(
            build_console_application(
                &compile_sources(&[source]).unwrap(),
                &ConsoleApplication {
                    id: [23; 16],
                    entry: "f".into()
                }
            )
            .is_err()
        );
    }
    for source in [
        "function f() as Toybox.Lang.Number or Null { return \"hi\"; }",
        "function f() as Toybox.Lang.String or Null { return 7; }",
        "function f() as Toybox.Lang.Number or Null { return true; }",
        "function f() as Toybox.Lang.Number or Null { return; }",
        "function f() as Toybox.Lang.String or Null { }",
        "function f() as Toybox.Lang.Number { return null; }",
        "function f() { return null; }",
        "function f() as Toybox.Lang.Boolean or Null { return null; }",
        "function f(x as Toybox.Lang.Number or Toybox.Lang.String) { return 0; }",
        "function f(x as Null or Toybox.Lang.Number) { return 0; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn user_arguments_use_union_assignability_and_results_remain_exact() {
    let functions = "module M { function number(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number or Null { return n; } function text(s as Toybox.Lang.String or Null) as Toybox.Lang.String or Null { return s; } function scalar(n as Toybox.Lang.Number) { return n; } }";
    for expression in [
        "M.number(null)",
        "M.number(7)",
        "M.number(\"7\".toNumber())",
        "M.text(null)",
        "M.text(\"hi\")",
        "M.text(\"hi\".substring(0, 1))",
    ] {
        let source = format!("function start() {{ {expression}; }}");
        let unit = compile_sources(&[&source, functions]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
    for expression in [
        "M.number(true)",
        "M.number(\"hi\")",
        "M.text(7)",
        "M.text(\"7\".toNumber())",
        "M.scalar(null)",
        "M.scalar(\"7\".toNumber())",
    ] {
        let source = format!("function start() {{ {expression}; }}");
        assert!(compile_sources(&[&source, functions]).is_err(), "{source}");
    }
    let source = "function start() { System.println(M.number(7)); }";
    assert!(compile_sources(&[source, functions]).is_err());
    // Forward and recursive call declarations preserve nullable results.
    let recursive = "module M { function first(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number or Null { return M.later(n); } function later(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number or Null { return M.first(n); } }";
    let unit = compile_sources(&[recursive]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn typed_and_text_construction_enforce_returns_and_guard_proofs() {
    let f = compile(
        "function f(x as Toybox.Lang.String or Null) as Toybox.Lang.String or Null { return x; }",
    )
    .unwrap();
    assert_eq!(
        f,
        Function::with_signature(
            "f".into(),
            vec![ParameterType::NullableString],
            ReturnType::NullableString,
            f.operations().to_vec()
        )
        .unwrap()
    );
    assert!(
        Function::with_signature(
            "f".into(),
            vec![ParameterType::NullableString],
            ReturnType::String,
            f.operations().to_vec()
        )
        .is_err()
    );
    assert!(
        parse_ir(
            &f.to_ir_text()
                .replace("result=nullable-string", "result=nullable-integer")
        )
        .is_err()
    );
    assert!(
        parse_ir(&f.to_ir_text().replace(
            "result=nullable-string",
            "result=nullable-string result=nullable-string"
        ))
        .is_err()
    );
    for source in [
        "function f(x as Toybox.Lang.String or Null) { return x.length(); }",
        "function f(x as Toybox.Lang.String or Null) { if (x == null) { return 0; } return x.length(); }",
        "function f(x as Toybox.Lang.String or Null) { var s = x; if (s == null) { return 0; } s = null; return s.length(); }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let guarded = compile("function f(x as Toybox.Lang.String or Null) { var s = x; if (s == null) { return 0; } return s.length(); }").unwrap();
    assert_eq!(guarded, parse_ir(&guarded.to_ir_text()).unwrap());
    let mut operations = guarded.operations().to_vec();
    for op in &mut operations {
        if let ciq_compiler::Operation::Branch {
            on_true, on_false, ..
        } = &mut op.operation
        {
            std::mem::swap(on_true, on_false);
        }
    }
    assert!(
        Function::with_parameter_types(
            "unsafe".into(),
            vec![ParameterType::NullableString],
            operations
        )
        .is_err()
    );
}
