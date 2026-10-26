#[path = "common/checking.rs"]
mod checking;

use ciq_compiler::{
    Function, Operation,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources_with_api, parse_function_ir, parse_function_ir_with_api,
    parse_unit_ir_with_api,
};

#[test]
fn unary_plus_preserves_complete_source_ir_assembly_and_prg_pipelines() {
    let ctx = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(2, 1, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-number-plus/app.mc"),
            include_str!("../../../examples/compiler-number-plus/plus.mc"),
        ],
        ctx,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), ctx).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [56; 16],
        entry: "MinimalConsoleApp.onStart".into(),
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
fn positive_is_checked_number_identity_with_no_vm_operation() {
    let source = "function f(n) { return +n; }";
    let f = compile_function(source).unwrap();
    assert_eq!(f.lower().encode().unwrap(), [0x35, 2, 0x12, 1, 0x16]);
    let positive = f
        .operations()
        .iter()
        .find(|op| matches!(op.operation, Operation::Positive { .. }))
        .unwrap();
    let span = positive.span.unwrap();
    assert_eq!(&source[span.start..span.end], "+n");
    assert!(f.to_ir_text().contains(".positive %1 %0"));
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    let api = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    assert_eq!(f, parse_function_ir_with_api(&f.to_ir_text(), api).unwrap());
    for expr in [
        "+0",
        "+2147483647",
        "+(-2147483648)",
        "+(+1)",
        "+(-1)",
        "+(~1)",
        "~(+1)",
        "-(+1)",
        "+/*sign*/1",
        "1 + +2",
    ] {
        let f = compile_function(&format!("function f() {{ return {expr}; }}")).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    for (source, expected) in [
        (
            "function f(a, b) { return +a * b; }",
            vec![0x35, 3, 0x12, 1, 0x12, 2, 0x05, 0x16],
        ),
        (
            "function f(a, b) { return +(a * b); }",
            vec![0x35, 3, 0x12, 1, 0x12, 2, 0x05, 0x16],
        ),
    ] {
        assert_eq!(
            compile_function(source).unwrap().lower().encode().unwrap(),
            expected
        );
    }
}

#[test]
fn positive_rejects_wrong_types_missing_proofs_and_unsupported_prefixes() {
    for expr in [
        "+true",
        "+'A'",
        "+\"text\"",
        "+null",
        "+[1]",
        "+new Toybox.WatchUi.View()",
        "+2147483648",
        "++1",
        "+ +1",
        "+-1",
        "+~1",
        "+!true",
    ] {
        assert!(
            checking::compile_function(&format!("function f() {{ return {expr}; }}")).is_err(),
            "{expr}"
        );
    }
    for source in [
        "function f(n) { return n++; }",
        "function f(n) { return n+++1; }",
        "function f(n as Toybox.Lang.Number or Null) { return +n; }",
        "function f(n as Toybox.Lang.Number or Null) { if (n != null) { n = null; return +n; } return 0; }",
    ] {
        assert!(checking::compile_function(source).is_err(), "{source}");
    }
    for source in [
        "function f(n as Toybox.Lang.Number or Null) { if (n != null) { return +n; } return 0; }",
        "function f(n as Toybox.Lang.Number or Null) { if (n == null) { return 0; } return +n; }",
        "function f() { var n = +(true || false).hashCode(); return n; }",
    ] {
        let f = compile_function(source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    let f = compile_function("function f(n) { return +n; }").unwrap();
    for (from, to) in [
        ("parameters=1", "parameters=1 parameter-types=boolean"),
        (
            "parameters=1",
            "parameters=1 parameter-types=nullable-integer",
        ),
        (".positive %1 %0", ".positive %1 %99"),
        (".positive %1 %0", ".positive %1"),
        (".positive %1 %0", ".positive %0 %0"),
        (".positive %1 %0", ".positive %1 %0 %2"),
    ] {
        let text = f.to_ir_text();
        assert!(text.contains(from));
        assert!(parse_function_ir(&text.replace(from, to)).is_err(), "{to}");
    }
    assert!(parse_function_ir("ciq-ir\n.function f\n.integer %0 +1\n.return %0\n.end\n").is_err());
}

#[test]
fn identity_keeps_recorded_reads_before_writes_and_ordinary_parser_limits() {
    let f = compile_function("function f() { var n = 1; var m = +n; n = 2; return m; }").unwrap();
    let mut ops = f.operations().to_vec();
    let assign = ops
        .iter()
        .position(|op| matches!(op.operation, Operation::LocalAssign { .. }))
        .unwrap();
    let moved = ops.drain(assign - 1..=assign).collect::<Vec<_>>();
    let positive = ops
        .iter()
        .position(|op| matches!(op.operation, Operation::Positive { .. }))
        .unwrap();
    ops.splice(positive..positive, moved);
    let error = Function::with_signature(
        "bad".into(),
        f.parameter_types().to_vec(),
        f.return_type(),
        ops,
    )
    .unwrap_err();
    assert!(
        error.message.contains("evaluation order"),
        "{}",
        error.message
    );
    let source = |n| {
        format!(
            "function f(n) {{ return {}n{}; }}",
            "+(".repeat(n),
            ")".repeat(n)
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
