use ciq_compiler::{
    Function, LocatedOperation, Operation, ParameterType, ReturnType, ValueId,
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources, parse_function_ir, parse_unit_ir,
};

#[test]
fn complete_app_preserves_typed_module_and_self_calls_through_text() {
    let unit = compile_sources(&[
        include_str!("../../../examples/compiler-lang-values/app.mc"),
        include_str!("../../../examples/compiler-lang-values/text.mc"),
    ])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains("parameter-types=boolean:string:string"));
    assert!(text.contains("result=string"));
    assert!(text.contains("result=boolean"));
    let resumed = parse_unit_ir(&text).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [19; 16],
        entry: "LangApp.onStart".into(),
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
}

#[test]
fn scalar_entries_generate_equivalent_wrappers_after_ir_resumption() {
    for body in [
        "function start() as Toybox.Lang.String { return \"hello\"; }",
        "function start() as Toybox.Lang.Boolean { return true; }",
    ] {
        let unit = compile_sources(&[body]).unwrap();
        let options = ConsoleApplication {
            id: [20; 16],
            entry: "start".into(),
        };
        assert_eq!(
            build_console_application(&unit, &options).unwrap(),
            build_console_application(&parse_unit_ir(&unit.to_ir_text()).unwrap(), &options)
                .unwrap()
        );
    }
}

#[test]
fn typed_function_construction_infers_supported_returns() {
    for (parameter, result) in [
        (ParameterType::String, ReturnType::String),
        (ParameterType::Boolean, ReturnType::Boolean),
    ] {
        let operations = vec![
            LocatedOperation {
                operation: Operation::ParameterRead {
                    result: ValueId(0),
                    index: 0,
                },
                span: None,
            },
            LocatedOperation {
                operation: Operation::Return { value: ValueId(0) },
                span: None,
            },
        ];
        let function =
            Function::with_parameter_types("identity".into(), vec![parameter], operations).unwrap();
        assert_eq!(function.return_type(), result);
        assert_eq!(function.parameter_types()[0].api_type(), result.api_type());
        assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
    }
    assert_eq!(
        compile_function("function f() as Toybox.Lang.String { return \"hi\"; }")
            .unwrap()
            .return_type(),
        ReturnType::String
    );
    assert_eq!(
        compile_function("function f() as Toybox.Lang.Boolean { return true; }")
            .unwrap()
            .return_type(),
        ReturnType::Boolean
    );
}

#[test]
fn forward_and_recursive_calls_use_declared_contracts() {
    let caller = "module A { function echo(text as Toybox.Lang.String) as Toybox.Lang.String { return B.echo(text); }
        function flag(value as Toybox.Lang.Boolean) as Toybox.Lang.Boolean { if (value) { return B.flag(false); } else { return false; } } }";
    let callee = "module B { function echo(text as Toybox.Lang.String) as Toybox.Lang.String { return text.toUpper(); }
        function flag(value as Toybox.Lang.Boolean) as Toybox.Lang.Boolean { return A.flag(value); } }";
    let unit = compile_sources(&[caller, callee]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert!(compile_sources(&[callee, caller]).is_ok());
}

#[test]
fn call_contracts_are_checked_against_callees_after_ir_edits() {
    let callee = "module M { function echo(text as Toybox.Lang.String) as Toybox.Lang.String { return text; } }";
    let caller = "function f() as Void { System.println(M.echo(\"hi\")); }";
    let unit = compile_sources(&[callee, caller]).unwrap();
    let text = unit.to_ir_text();
    for bad in [
        text.replace("result=string", "result=boolean"),
        text.replace("parameter-types=string", "parameter-types=boolean"),
        text.replace("result=string", "result=void"),
    ] {
        assert!(parse_unit_ir(&bad).is_err(), "{bad}");
    }
    for statement in [
        "M.echo(true);",
        "M.echo(1);",
        "System.println(M.echo(false));",
    ] {
        let caller = format!("function f() as Void {{ {statement} }}");
        let error = compile_sources(&[callee, &caller]).unwrap_err();
        assert_eq!(error.source, 1);
        assert!(
            error.diagnostic.message.contains("argument type"),
            "{error}"
        );
    }
    let wrong_self = "class C extends Application.AppBase {
        function f() { self.echo(true); }
        function echo(s as Toybox.Lang.String) as Toybox.Lang.String { return s; } }";
    assert!(
        compile_sources(&[wrong_self])
            .unwrap_err()
            .diagnostic
            .message
            .contains("argument type")
    );
}

#[test]
fn annotated_returns_remain_uniform_and_callback_contracts_remain_separate() {
    for source in [
        "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.String { if (flag) { return \"hi\"; } else { return 1; } }",
        "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.String { if (flag) { return \"hi\"; } }",
        "function f() as Toybox.Lang.Boolean { return \"hi\"; }",
        "function f() as Void { return true; }",
        "function f() as Toybox.Lang.String { return; }",
        "function f(s as Toybox.Lang.String) as Toybox.Lang.Number { return s + 1; }",
        "function f(s as Toybox.Lang.String) as Toybox.Lang.Number { if (s) { return 1; } else { return 2; } }",
        "function f(s as Toybox.Lang.Boolean) as Toybox.Lang.Number { return s + 1; }",
        "function f(s as Toybox.Lang.Unknown) as Toybox.Lang.Number { return 1; }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    assert!(compile_sources(&["class C extends Application.AppBase { function onStart(state as Toybox.Lang.String) { } }"]).is_err());
    assert!(compile_sources(&["class C extends Application.AppBase { function onStop(state) as Toybox.Lang.Boolean { return true; } }"]).is_err());
}
