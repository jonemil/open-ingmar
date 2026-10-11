use ciq_compiler::{
    Function, Operation, ValueId,
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, parse_ir, parse_unit_ir,
};

#[test]
fn inherited_number_calls_preserve_complete_application_and_text_paths() {
    let unit = compile_sources(&[
        include_str!("../../../examples/compiler-native-calls/app.mc"),
        include_str!("../../../examples/compiler-native-calls/numbers.mc"),
    ])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".call-native"));
    let options = ConsoleApplication {
        id: [7; 16],
        entry: "start".into(),
    };
    let native = build_console_application(&unit, &options).unwrap();
    let resumed = build_console_application(&parse_unit_ir(&text).unwrap(), &options).unwrap();
    assert_eq!(native, resumed);
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    assert!(assembly.to_text().contains("getsv @__ciq_native_toString"));
    let function = compile("function show(n) { System.println(n.toString()); }").unwrap();
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
}

#[test]
fn catalogue_knowledge_does_not_enable_other_native_calls() {
    for (expression, message) in [
        ("n.toString(1)", "argument count"),
        ("n.abs()", "no supported lowering"),
        ("n.missing()", "not known"),
    ] {
        let error = compile(&format!(
            "function show(n) {{ System.println({expression}); }}"
        ))
        .unwrap_err();
        assert!(error.message.contains(message), "{error}");
    }
    for source in [
        "function f() { var s = \"hello\"; System.println(s.toString()); }",
        "function f() { var n = null; System.println(n.toString()); }",
        "function f(n) { return n.toString(); }",
        "function f(n) { var x = n.toString() + 1; return x; }",
        "function f(n) { var x = n.toString(); x = 1; }",
        "function f(n) { n.toString(); }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    assert!(
        compile_sources(&["class C extends Application.AppBase {
        function onStart(state) { System.println(state.toString()); } }"])
        .is_err()
    );
}

#[test]
fn native_ir_revalidates_receiver_member_and_effect_order() {
    let valid = compile("function f(n) { System.println(n.toString()); }")
        .unwrap()
        .to_ir_text();
    for bad in [
        valid.replace("toString", "abs"),
        valid.replace("toString", "missing"),
        valid.replace("toString", "toString %0"),
        valid.replace(".parameter %0 0", ".string %0 \"bad\""),
    ] {
        assert!(parse_ir(&bad).is_err(), "{bad}");
    }
    // The receiver call and conversion are observable operations. Printing a
    // later-defined integer first must not move those calls after that print.
    let operations = vec![
        Operation::ModuleCall {
            result: ValueId(0),
            module: "M".into(),
            member: "next".into(),
            arguments: vec![],
        },
        Operation::NativeCall {
            result: ValueId(1),
            receiver: ValueId(0),
            member: "toString".into(),
            arguments: vec![],
        },
        Operation::Integer {
            result: ValueId(2),
            value: 1,
        },
        Operation::Println { value: ValueId(2) },
        Operation::Println { value: ValueId(1) },
        Operation::ReturnVoid,
    ];
    assert!(
        Function::new(
            "bad".into(),
            operations
                .into_iter()
                .map(|operation| ciq_compiler::LocatedOperation {
                    operation,
                    span: None
                })
                .collect()
        )
        .unwrap_err()
        .message
        .contains("order")
    );
}
