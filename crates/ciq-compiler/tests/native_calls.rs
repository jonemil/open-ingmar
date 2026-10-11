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
    assert!(assembly.to_text().contains("getsv @__ciq_native_format"));
    assert!(assembly.to_text().contains("getsv @__ciq_native_abs"));
    for member in ["length", "toLower", "toUpper"] {
        assert!(
            assembly
                .to_text()
                .contains(&format!("getsv @__ciq_native_{member}"))
        );
    }
    let function = compile("function show(n) { System.println(n.toString()); }").unwrap();
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
}

#[test]
fn string_methods_preserve_contracts_and_explicit_bindings() {
    for source in [
        "function f() { return \"é猫\".length(); }",
        "function f() { return \"\".length() + 1; }",
        "function f() { System.println(41.format(\"%d\").toUpper().length().toString()); }",
        "function f() { var s = \"Hello\"; s = s.toLower(); System.println(s.toUpper()); }",
        "function f() { System.println(\"text\".toString()); }",
        "function f() { if (\"text\".length() > 0) { return 1; } else { return 0; } }",
    ] {
        let function = compile(source).unwrap();
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
        function
            .validate_api(
                ciq_compiler::api::ApiContext::new(
                    ciq_compiler::api::ApiLevel(1, 0, 0),
                    ciq_compiler::api::ApiLevel(1, 0, 0),
                )
                .unwrap(),
            )
            .unwrap();
    }
    for (member, id) in [
        ("length", 0x008000bb),
        ("toLower", 0x008000be),
        ("toUpper", 0x008000bf),
        ("toString", 0x008000af),
    ] {
        let function = compile(&format!(
            "function f() {{ System.println(\"Hi\".{member}()); }}"
        ))
        .unwrap();
        let assembly = function.lower();
        let symbols = ciq_assembly::Program::parse(&format!(
            ".symbol native \"{member}\" id=0x{id:08x}\n.symbol system \"Toybox_System\" id=0x00800005\n.symbol println \"println\" id=0x00800067"
        )).unwrap();
        let bindings = ciq_compiler::symbol_bindings(&assembly, &symbols).unwrap();
        assert_eq!(bindings[&format!("__ciq_native_{member}")], id);
        assert!(
            assembly
                .to_text()
                .contains(&format!("getsv @__ciq_native_{member}\ninvokemz"))
        );
    }
    // Result contracts must be rechecked after an IR edit, not inferred from
    // the consumer's expected type.
    let text = compile("function f() { return \"Hi\".length() + 1; }")
        .unwrap()
        .to_ir_text();
    assert!(parse_ir(&text.replace("length", "toUpper")).is_err());
    for source in [
        "function f() { System.println(1.length()); }",
        "function f() { System.println(true.toUpper()); }",
        "function f() { System.println(\"Hi\".length(1)); }",
        "function f() { System.println(\"Hi\".toLower(\"x\")); }",
        "function f() { System.println(\"Hi\".toUpper(null)); }",
        "function f() { return \"Hi\".toLower(); }",
        "function f() { System.println(\"Hi\".toUpper() + 1); }",
        "function f() { System.println(\"Hi\".find(\"i\")); }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn catalogue_knowledge_does_not_enable_other_native_calls() {
    for (expression, message) in [
        ("n.toString(1)", "argument count"),
        ("n.toChar()", "no supported lowering"),
        ("n.abs(1)", "argument count"),
        ("n.missing()", "not known"),
        ("n.format()", "argument count"),
        ("n.format(1)", "argument types"),
        ("n.format(null)", "argument types"),
        ("n.format(\"%d\", \"%d\")", "argument count"),
    ] {
        let error = compile(&format!(
            "function show(n) {{ System.println({expression}); }}"
        ))
        .unwrap_err();
        assert!(error.message.contains(message), "{error}");
    }
    for source in [
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
fn number_results_support_arithmetic_chained_calls_and_integer_returns() {
    for source in [
        "function f(n) { return n.abs(); }",
        "function f(n) { return n.abs() + 1; }",
        "function f(n) { var magnitude = n.abs(); return magnitude - 1; }",
        "function f(n) { System.println(n.abs().abs().format(\"%03d\")); }",
        "function f(n) { if (n.abs() > 0) { return 1; } else { return 0; } }",
        "function f(n) { return M.consume(n.abs()); }",
    ] {
        let function = compile(source).unwrap();
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    }
    let function = compile("function f(n) { return n.abs(); }").unwrap();
    assert_eq!(function.return_type(), ciq_compiler::ReturnType::Integer);
    let assembly = function.lower();
    let text = assembly.to_text();
    assert!(text.contains("getsv @__ciq_native_abs\ninvokemz\nreturn"));
    let symbols = ciq_assembly::Program::parse(".symbol abs \"abs\" id=0x008000a6").unwrap();
    let bindings = ciq_compiler::symbol_bindings(&assembly, &symbols).unwrap();
    assert_eq!(bindings["__ciq_native_abs"], 0x008000a6);
    let code = assembly
        .encode_with_references(&bindings, &Default::default())
        .unwrap();
    let resumed = parse_ir(&function.to_ir_text()).unwrap().lower();
    assert_eq!(
        code,
        resumed
            .encode_with_references(&bindings, &Default::default())
            .unwrap()
    );
    // Changing a numeric-result call to a String-result call invalidates
    // consuming arithmetic, including after text edits.
    let arithmetic = compile("function f(n) { return n.abs() + 1; }").unwrap();
    assert!(parse_ir(&arithmetic.to_ir_text().replace("abs", "toString")).is_err());
    assert!(compile("function f(n) { return n.abs().format(\"%d\"); }").is_err());
    assert!(compile("function f(n) { System.println(n.abs(1)); }").is_err());
    assert!(compile("function f() { System.println(\"bad\".abs()); }").is_err());
}

#[test]
fn computed_receivers_and_format_arguments_preserve_types_and_precedence() {
    for source in [
        "function f() { System.println((40 + 2).format(\"%03d\")); }",
        "function f() { System.println(41.toString()); }",
        "function f(n) { var fmt = \"%d\"; var text = n.format(fmt); System.println(text); }",
        "function f(n) { System.println((n - 42).toString()); }",
        "function f() { System.println(M.value().format(M.arg().toString())); }",
    ] {
        let native = compile(source).unwrap();
        let resumed = parse_ir(&native.to_ir_text()).unwrap();
        assert_eq!(native.lower(), resumed.lower());
    }
    let function = compile("function f() { System.println((40 + 2).format(\"%03d\")); }").unwrap();
    let assembly = function.assembly();
    let addition = assembly.find("addv").unwrap();
    let member = assembly.find("getsv @__ciq_native_format").unwrap();
    let argument = assembly[member..].find("news").unwrap() + member;
    let invoke = assembly[argument..].find("invokem 0x02").unwrap() + argument;
    assert!(addition < member && member < argument && argument < invoke);
    for source in [
        "function f() { System.println((40 + 2).format(1)); }",
        "function f() { System.println(41.format(\"%d\") + 1); }",
        "function f() { System.println(41.format); }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn format_ir_rejects_bad_types_and_argument_receiver_reordering() {
    let function =
        compile("function f() { System.println(M.value().format(M.arg().toString())); }").unwrap();
    let operations = function.operations().to_vec();
    let receiver = operations
        .iter()
        .position(|op| {
            matches!(&op.operation,
        Operation::ModuleCall { member, .. } if member == "value")
        })
        .unwrap();
    let argument = operations
        .iter()
        .position(|op| {
            matches!(&op.operation,
        Operation::ModuleCall { member, .. } if member == "arg")
        })
        .unwrap();
    let mut reordered = operations;
    reordered.swap(receiver, argument);
    assert!(
        Function::new("reordered".into(), reordered)
            .unwrap_err()
            .message
            .contains("order")
    );

    let text = compile("function f(n) { System.println(n.format(\"%d\")); }")
        .unwrap()
        .to_ir_text();
    let string_line = text
        .lines()
        .find(|line| line.starts_with(".string"))
        .unwrap();
    let result = string_line.split_whitespace().nth(1).unwrap();
    let bad = text.replace(string_line, &format!(".integer {result} 1"));
    assert!(
        parse_ir(&bad)
            .unwrap_err()
            .message
            .contains("argument types")
    );
    let deep = format!(
        "function f(n) {{ System.println(n{}); }}",
        ".toString()".repeat(70)
    );
    assert!(compile(&deep).unwrap_err().message.contains("nesting"));
}

#[test]
fn positional_call_nesting_keeps_the_existing_bound() {
    for (prefix, inner) in [("M.value(", "1"), ("41.format(", "\"%d\"")] {
        let source = |depth| {
            format!(
                "function f() {{ var value = {}{}{}; System.println(value); }}",
                prefix.repeat(depth),
                inner,
                ")".repeat(depth)
            )
        };
        assert!(compile(&source(64)).is_ok());
        assert!(
            compile(&source(65))
                .unwrap_err()
                .message
                .contains("nesting")
        );
    }
}

#[test]
fn native_ir_revalidates_receiver_member_and_effect_order() {
    let valid = compile("function f(n) { System.println(n.toString()); }")
        .unwrap()
        .to_ir_text();
    for bad in [
        valid.replace("toString", "toChar"),
        valid.replace("toString", "missing"),
        valid.replace("toString", "toString %0"),
        valid.replace(".parameter %0 0", ".null %0"),
    ] {
        assert!(parse_ir(&bad).is_err(), "{bad}");
    }
    // The receiver call and conversion are observable operations. Printing a
    // later-defined integer first must not move those calls after that print.
    let operations = vec![
        Operation::ModuleCall {
            result: ValueId(0),
            result_type: ciq_compiler::ReturnType::Integer,
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
