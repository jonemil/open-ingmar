use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources_with_api, parse_function_ir, parse_unit_ir_with_api,
};

#[test]
fn number_negation_preserves_complete_pipelines_precedence_and_single_evaluation() {
    let context = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-number-negation/app.mc"),
            include_str!("../../../examples/compiler-number-negation/numbers.mc"),
        ],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [33; 16],
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
    let f = compile_function("function f() { return -M.value(); }").unwrap();
    let assembly = f.lower().to_text();
    assert_eq!(assembly.matches("getmv").count(), 1);
    assert!(assembly.contains("ipushz\ngetmv"));
    assert!(assembly.contains("invokemz\nsubv"));
    for source in [
        "function f(n) { return -n.abs(); }",
        "function f(n) { return -(n + 1); }",
        "function f(n) { return -(-n); }",
        "function f(n) { return -n + 1; }",
    ] {
        let f = compile_function(source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    let f = compile_function("function f(n) { return -n.abs(); }").unwrap();
    let text = f.to_ir_text();
    assert!(text.find(".call-native").unwrap() < text.find(".negate").unwrap());
    let f = compile_function("function f(n) { return -n + 1; }").unwrap();
    assert!(f.to_ir_text().find(".negate").unwrap() < f.to_ir_text().find(".add").unwrap());
    let f = compile_function("function f() { return -(-2147483648); }").unwrap();
    assert_eq!(
        f.lower().encode().unwrap(),
        [0x35, 1, 0x37, 0x25, 0x80, 0, 0, 0, 0x04, 0x16]
    );
}

#[test]
fn negation_requires_a_number_and_rechecks_guards_spans_and_effect_order() {
    for source in [
        "function f() { return -true; }",
        "function f() { return -'A'; }",
        "function f() { return -\"a\"; }",
        "function f() { return -null; }",
        "function f(n as Toybox.Lang.Number or Null) { return -n; }",
        "function f() { return -1.toNumber(); }",
        "function f() { return +true; }",
        "function f(n) { return --n; }",
        "function f(n) { return - -n; }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    let f = compile_function("function f(n) { return -n; }").unwrap();
    let op = f
        .operations()
        .iter()
        .find(|op| matches!(op.operation, ciq_compiler::Operation::Negate { .. }))
        .unwrap();
    assert_eq!(
        &"function f(n) { return -n; }"[op.span.unwrap().start..op.span.unwrap().end],
        "-n"
    );
    assert!(
        parse_function_ir(
            &f.to_ir_text()
                .replace(".parameter %0 0", ".boolean %0 true")
        )
        .is_err()
    );
    let f = compile_function(
        "function f(n as Toybox.Lang.Number or Null) { if (n == null) { return 0; } return -n; }",
    )
    .unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
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
    let f = compile_function("function f() { System.println(-M.first()); return M.second(); }")
        .unwrap();
    let mut operations = f.operations().to_vec();
    let negation = operations
        .iter()
        .position(|op| matches!(op.operation, ciq_compiler::Operation::Negate { .. }))
        .unwrap();
    let second = operations.iter().position(|op| matches!(&op.operation, ciq_compiler::Operation::ModuleCall { member, .. } if member=="second")).unwrap();
    let moved = operations.remove(second);
    operations.insert(negation, moved);
    let error = ciq_compiler::Function::new("bad".into(), operations).unwrap_err();
    assert!(
        error.message.contains("evaluation order"),
        "{}",
        error.message
    );
    let source = |depth: usize| {
        format!(
            "function f(n) {{ return {}n{}; }}",
            "-(".repeat(depth),
            ")".repeat(depth)
        )
    };
    compile_function(&source(32)).unwrap();
    assert!(
        compile_function(&source(33))
            .unwrap_err()
            .message
            .contains("nesting")
    );
}
