use ciq_compiler::{
    Operation, ParameterType, ReturnType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources_with_api, parse_ir, parse_unit_ir_with_api,
};

#[test]
fn boolean_negation_preserves_complete_pipelines_and_precedence() {
    let context = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-boolean-negation/app.mc"),
            include_str!("../../../examples/compiler-boolean-negation/flags.mc"),
        ],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [34; 16],
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
    for expression in [
        "!b",
        "!!b",
        "!b.equals(false)",
        "(!b).equals(false)",
        "!(1 > 2)",
        "!b == false",
    ] {
        let f = compile(&format!(
            "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.Boolean {{ return {expression}; }}"
        ))
        .unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    let f = compile(
        "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.Boolean { return !b.equals(false); }",
    )
    .unwrap();
    let ir = f.to_ir_text();
    assert!(ir.find(".call-native").unwrap() < ir.find(".not").unwrap());
    assert_eq!(f.lower().to_text().matches("getsv").count(), 1);
    let f = compile("function f() as Toybox.Lang.Boolean { return !true; }").unwrap();
    assert_eq!(
        f.lower().encode().unwrap(),
        [0x35, 1, 0x3e, 0x3f, 0x1a, 0x16]
    );
}

#[test]
fn logical_negation_rechecks_types_dependencies_spans_and_null_proofs() {
    for expression in [
        "!1",
        "!'A'",
        "!\"a\"",
        "!null",
        "!-1",
        "!true + 1",
        "1 !== 2",
    ] {
        assert!(
            compile(&format!(
                "function f() as Toybox.Lang.Boolean {{ return {expression}; }}"
            ))
            .is_err(),
            "{expression}"
        );
    }
    let source = "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.Boolean { return !b; }";
    let f = compile(source).unwrap();
    let op = f
        .operations()
        .iter()
        .find(|op| matches!(op.operation, Operation::Not { .. }))
        .unwrap();
    let span = op.span.unwrap();
    assert_eq!(&source[span.start..span.end], "!b");
    let mut operations = f.operations().to_vec();
    operations[0].operation = Operation::Integer {
        result: ciq_compiler::ValueId(0),
        value: 1,
    };
    assert!(
        ciq_compiler::Function::with_signature(
            "bad".into(),
            vec![ParameterType::Boolean],
            ReturnType::Boolean,
            operations
        )
        .is_err()
    );
    assert!(parse_ir(&f.to_ir_text().replace(".parameter %0 0", ".integer %0 1")).is_err());
    assert!(parse_ir(&f.to_ir_text().replace(".not %1 %0", ".not %1 %99")).is_err());
    let guarded = compile("function f(n as Toybox.Lang.Number or Null) { if (!(n == null)) { return n + 1; } return 0; }").unwrap();
    assert_eq!(guarded, parse_ir(&guarded.to_ir_text()).unwrap());
    let f = compile("function f() as Toybox.Lang.Boolean { System.println(!true.equals(false)); return true.equals(true); }").unwrap();
    let mut operations = f.operations().to_vec();
    let negation = operations
        .iter()
        .position(|op| matches!(op.operation, Operation::Not { .. }))
        .unwrap();
    let second = operations
        .iter()
        .rposition(|op| matches!(op.operation, Operation::NativeCall { .. }))
        .unwrap();
    let moved = operations.drain(second - 2..=second).collect::<Vec<_>>();
    operations.splice(negation..negation, moved);
    let error = ciq_compiler::Function::with_signature(
        "bad".into(),
        vec![],
        ReturnType::Boolean,
        operations,
    )
    .unwrap_err();
    assert!(
        error.message.contains("evaluation order"),
        "{}",
        error.message
    );
    let nested = |n: usize| {
        format!(
            "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.Boolean {{ return {}b; }}",
            "!".repeat(n)
        )
    };
    compile(&nested(64)).unwrap();
    assert!(
        compile(&nested(65))
            .unwrap_err()
            .message
            .contains("nesting")
    );
}
