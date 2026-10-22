use ciq_compiler::{
    Function, Operation, ReturnType,
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources, parse_function_ir, parse_unit_ir,
};

#[test]
fn char_increment_preserves_type_across_source_saved_ir_and_direct_addition() {
    for (target, setup, result) in [
        ("c", "var c = 'a';", "c"),
        ("c[0]", "var c = ['a'];", "c[0]"),
    ] {
        for update in [format!("{target}++"), format!("++{target}")] {
            let source = format!(
                "function f() as Toybox.Lang.Char {{ {setup} {update}; return {result}; }}"
            );
            let f = compile_function(&source).unwrap();
            assert_eq!(f.return_type(), ReturnType::Char);
            let text = f.to_ir_text();
            assert_eq!(f, parse_function_ir(&text).unwrap());
            assert!(parse_function_ir(&text.replace(".increment", ".decrement")).is_err());
            assert_eq!(
                f.lower(),
                compile_function(&source.replace(&update, &format!("{target} += 1")))
                    .unwrap()
                    .lower()
            );
            let old = ciq_compiler::api::ApiContext::new(
                ciq_compiler::api::ApiLevel(1, 2, 9),
                ciq_compiler::api::ApiLevel(1, 0, 0),
            )
            .unwrap();
            assert!(ciq_compiler::compile_function_with_api(&source, old).is_err());
            assert!(ciq_compiler::parse_function_ir_with_api(&text, old).is_err());
            let mut operations = f.operations().to_vec();
            for op in &mut operations {
                if let Operation::Step { increment, .. } = &mut op.operation {
                    *increment = false;
                }
            }
            assert!(
                Function::with_signature("f".into(), vec![], ReturnType::Char, operations).is_err()
            );
        }
    }
}

#[test]
fn prefix_and_postfix_statements_use_direct_number_steps_and_existing_slots() {
    for (update, compound, directive) in [
        ("n++", "n += 1", ".increment"),
        ("++n", "n += 1", ".increment"),
        ("n--", "n -= 1", ".decrement"),
        ("--n", "n -= 1", ".decrement"),
    ] {
        let source = format!("function f() {{ var n = 1; {update}; return n; }}");
        let f = compile_function(&source).unwrap();
        let text = f.to_ir_text();
        assert!(text.contains(directive));
        assert_eq!(f, parse_function_ir(&text).unwrap());
        let assembly = f.lower();
        assert_eq!(
            assembly,
            ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
        );
        assert_eq!(
            assembly.encode().unwrap(),
            compile_function(&source.replace(update, compound))
                .unwrap()
                .lower()
                .encode()
                .unwrap()
        );
        let op = f
            .operations()
            .iter()
            .find(|op| matches!(op.operation, Operation::Step { .. }))
            .unwrap();
        let span = op.span.unwrap();
        assert_eq!(&source[span.start..span.end], format!("{update};"));
    }
    let parameters = (0..254)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    compile_function(&format!(
        "function f({parameters}) {{ var n = 1; n++; --n; return n; }}"
    ))
    .unwrap();
}

#[test]
fn updates_reject_unsupported_targets_expression_uses_and_invalid_saved_steps() {
    for source in [
        "function f(n as Toybox.Lang.String) { n++; }",
        "function f() { unknown++; }",
        "function f() { var n = 1; (n)++; }",
        "function f() { var n = 1; ++(n); }",
        "function f() { var n = true; ++n; }",
        "function f() { var n = 'a'; n--; }",
        "function f() { var n = 'a'; --n; }",
        "function f() { var n = \"a\"; n++; }",
        "function f() { var n = [1]; --n; }",
        "function f() { var n = ['a']; n[0]--; }",
        "function f() { var n = ['a']; --n[0]; }",
        "function f() { var n = \"1\".toNumber(); n++; }",
        "function f() { var n = 1; return n++; }",
        "function f() { var n = 1; return ++n; }",
        "function f() { var n = 1; var value = n++; }",
        "function f() { var n = 1; System.println(++n); }",
        "function f() { var n = 1; while (n++ < 2) {} }",
        "function f() { 1++; }",
        "function f() { var n = 1; n++++; }",
        "function f() { var n = 1; ++n++; }",
        "function f() { var n = 1; n++ = 2; }",
        "function f() { var n = [1]; n[true]++; }",
        "function f() { var n = 1; if (true) { var inner = 2; } inner++; }",
    ] {
        assert!(compile_function(source).is_err(), "accepted {source}");
    }
    let text = "ciq-ir\n.function f\n.integer %0 1\n.increment %1 %0\n.return %1\n.end\n";
    let f = parse_function_ir(text).unwrap();
    assert_eq!(
        f.lower().encode().unwrap(),
        [0x35, 1, 0x38, 1, 0x38, 1, 0x03, 0x16]
    );
    assert_eq!(
        parse_function_ir(&text.replace(".increment", ".decrement"))
            .unwrap()
            .lower()
            .encode()
            .unwrap(),
        [0x35, 1, 0x38, 1, 0x38, 1, 0x04, 0x16]
    );
    for bad in [
        text.replace(".integer %0 1", ".boolean %0 true"),
        text.replace("%1 %0", "%1 %99"),
        text.replace("%1 %0", "%1 %0 %0"),
        text.replace(".return %1", ".return %0"),
    ] {
        assert!(parse_function_ir(&bad).is_err());
    }
    let mut operations = f.operations().to_vec();
    operations[0].operation = Operation::Boolean {
        result: ciq_compiler::ValueId(0),
        value: true,
    };
    assert!(Function::with_signature("f".into(), vec![], ReturnType::Integer, operations).is_err());
    let guarded =
        compile_function("function f() { var n = \"7\".toNumber(); if (n != null) { n++; } }")
            .unwrap();
    assert_eq!(guarded, parse_function_ir(&guarded.to_ir_text()).unwrap());
}

#[test]
fn indexed_updates_capture_once_in_order_and_keep_branching_indices_and_frame_limits() {
    let f = compile_function("function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a[(true || false).hashCode() - 1]++; }").unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    let unit = compile_sources(&["module M { function array(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Array<Toybox.Lang.Number> { return a; } function index() { return 0; } } function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { M.array(a)[M.index()]++; }"]).unwrap();
    let f = unit
        .declarations()
        .iter()
        .find(|d| d.function.name() == "f")
        .unwrap();
    let calls: Vec<_> = f
        .function
        .operations()
        .iter()
        .filter_map(|op| {
            if let Operation::ModuleCall { member, .. } = &op.operation {
                Some(member.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(calls, ["array", "index"]);
    assert_eq!(
        f.function
            .operations()
            .iter()
            .filter(|op| matches!(op.operation, Operation::ArrayRead { .. }))
            .count(),
        1
    );
    assert_eq!(
        f.function
            .operations()
            .iter()
            .filter(|op| matches!(op.operation, Operation::ArrayWrite { .. }))
            .count(),
        1
    );
    for (count, ok) in [(253, true), (254, false)] {
        let parameters = std::iter::once("a as Toybox.Lang.Array<Toybox.Lang.Number>".to_owned())
            .chain((1..count).map(|i| format!("p{i}")))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(
            compile_function(&format!("function f({parameters}) {{ a[0]++; }}")).is_ok(),
            ok
        );
    }
    // A declared parameter remains immutable even though its array elements can change.
    assert!(
        compile_function("function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a++; }")
            .is_err()
    );
}

#[test]
fn complete_update_application_preserves_source_ir_assembly_and_prg_paths() {
    let unit =
        compile_sources(&[include_str!("../../../examples/compiler-increments/app.mc")]).unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let app = ConsoleApplication {
        id: [0x86; 16],
        entry: "MinimalConsoleApp.onStart".into(),
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
}
