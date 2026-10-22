use ciq_compiler::{
    Function, Operation,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources_with_api, parse_function_ir, parse_unit_ir_with_api,
};

fn context() -> ApiContext {
    ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(2, 1, 0)).unwrap()
}

#[test]
fn compound_targets_preserve_complete_pipelines() {
    let ctx = context();
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-compound-targets/app.mc"),
            include_str!("../../../examples/compiler-compound-targets/targets.mc"),
        ],
        ctx,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), ctx).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [58; 16],
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
fn number_fields_share_read_binary_write_lowering_for_all_operators() {
    let source = |body| {
        format!(
            "using Toybox.WatchUi; class Counter extends WatchUi.View {{ var count = 12; function initialize() {{ View.initialize(); }} function run() {{ {body} }} }}"
        )
    };
    for op in ["+", "-", "*", "/", "%", "&", "|", "^", "<<", ">>"] {
        let compound = source(format!("self.count {op}= 3;"));
        let expanded = source(format!("self.count = self.count {op} 3;"));
        let unit = compile_sources_with_api(&[&compound], context()).unwrap();
        let explicit = compile_sources_with_api(&[&expanded], context()).unwrap();
        assert_eq!(unit.lower(), explicit.lower(), "{op}=");
        assert_eq!(
            unit,
            parse_unit_ir_with_api(&unit.to_ir_text(), context()).unwrap()
        );
    }
    for body in [
        "self.count += true;",
        "self.count += 'A';",
        "self.count += \"text\";",
        "self.missing += 1;",
    ] {
        assert!(
            compile_sources_with_api(&[&source(body.into())], context()).is_err(),
            "{body}"
        );
    }
}

#[test]
fn indexed_compounds_capture_location_and_read_once() {
    for op in ["+", "-", "*", "/", "%", "&", "|", "^", "<<", ">>"] {
        let source = format!(
            "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>, i, n) {{ a[i] {op}= n; return 0; }}"
        );
        let f = compile_function(&source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        let ops = f.operations();
        assert_eq!(
            ops.iter()
                .filter(|op| matches!(op.operation, Operation::LocalInitialize { .. }))
                .count(),
            2
        );
        for index in 0..3 {
            assert_eq!(ops.iter().filter(|op| matches!(op.operation, Operation::ParameterRead { index: at, .. } if at == index)).count(), 1);
        }
        let decoded = ciq_mc::decode(&f.lower().encode().unwrap()).unwrap();
        for opcode in [ciq_mc::Opcode::Agetv, ciq_mc::Opcode::Aputv] {
            assert_eq!(
                decoded
                    .iter()
                    .filter(|op| op.instruction.opcode() == opcode)
                    .count(),
                1
            );
        }
        let read_span = ops
            .iter()
            .find_map(|op| {
                matches!(op.operation, Operation::ArrayRead { .. }).then_some(op.span.unwrap())
            })
            .unwrap();
        assert_eq!(&source[read_span.start..read_span.end], "a[i]");
        let write_span = ops
            .iter()
            .find_map(|op| {
                matches!(op.operation, Operation::ArrayWrite { .. }).then_some(op.span.unwrap())
            })
            .unwrap();
        assert_eq!(
            &source[write_span.start..write_span.end],
            format!("a[i] {op}= n;")
        );
    }
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) { a[0] += 1; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>, n as Toybox.Lang.Number or Null) { if (n != null) { a[0] += n; } return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a[(false || true).hashCode()] += (true && false).hashCode(); return 0; }",
        "function f() { [1, 2][0] += 1; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a[-1] += 2; return 0; }",
    ] {
        let f = compile_function(source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a[0] += 'A'; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a[0] += null; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a[true] += 1; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) { a[0] += 'A'; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) { a[0] -= 1; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.WatchUi.View>) { a[0] += 1; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>, n as Toybox.Lang.Number or Null) { a[0] += n; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { return a[0] += 1; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a[0][0] += 1; return 0; }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
}

#[test]
fn indexed_reads_stay_before_rhs_calls_and_branching_effects() {
    let f = compile_function(
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a[0] += M.value(); return 0; }",
    )
    .unwrap();
    let mut ops = f.operations().to_vec();
    let read = ops
        .iter()
        .position(|op| matches!(op.operation, Operation::ArrayRead { .. }))
        .unwrap();
    let call = ops
        .iter()
        .position(|op| matches!(op.operation, Operation::ModuleCall { .. }))
        .unwrap();
    ops.swap(read, call);
    let err = Function::with_signature(
        "bad".into(),
        f.parameter_types().to_vec(),
        f.return_type(),
        ops,
    )
    .unwrap_err();
    assert!(err.message.contains("evaluation order"), "{}", err.message);
    let f = compile_function("function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a[0] += (false || true).hashCode(); return 0; }").unwrap();
    let ops = f.operations();
    let read = ops
        .iter()
        .position(|op| matches!(op.operation, Operation::ArrayRead { .. }))
        .unwrap();
    let branch = ops
        .iter()
        .position(|op| matches!(op.operation, Operation::Branch { .. }))
        .unwrap();
    assert!(read < branch);
}

#[test]
fn capture_locals_use_the_frame_budget_without_reducing_parser_nesting() {
    let params = |count| {
        std::iter::once("a as Toybox.Lang.Array<Toybox.Lang.Number>".into())
            .chain((1..count).map(|n| format!("p{n}")))
            .collect::<Vec<String>>()
            .join(",")
    };
    compile_function(&format!(
        "function f({}) {{ a[0] += 1; return 0; }}",
        params(253)
    ))
    .unwrap();
    let err = compile_function(&format!(
        "function f({}) {{ a[0] += 1; return 0; }}",
        params(254)
    ))
    .unwrap_err();
    assert!(err.message.contains("255-slot limit"), "{}", err.message);
    let nested = |depth: usize| {
        format!(
            "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) {{ a[0] += {}1{}; return 0; }}",
            "M.value(".repeat(depth),
            ")".repeat(depth)
        )
    };
    compile_function(&nested(64)).unwrap();
    assert!(
        compile_function(&nested(65))
            .unwrap_err()
            .message
            .contains("nesting")
    );
}
