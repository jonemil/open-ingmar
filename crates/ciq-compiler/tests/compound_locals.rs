use ciq_compiler::{
    Function, Operation,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources_with_api, parse_function_ir, parse_unit_ir_with_api,
};

#[test]
fn local_compounds_preserve_complete_pipelines() {
    let ctx = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(2, 1, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-compound-locals/app.mc"),
            include_str!("../../../examples/compiler-compound-locals/compound.mc"),
        ],
        ctx,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), ctx).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [57; 16],
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
fn every_operator_matches_explicit_read_binary_write_lowering() {
    for op in ["+", "-", "*", "/", "%", "&", "|", "^", "<<", ">>"] {
        let compound = compile_function(&format!(
            "function f() as Toybox.Lang.Number {{ var n = 12; n {op}= 3; return n; }}"
        ))
        .unwrap();
        let expanded = compile_function(&format!(
            "function f() as Toybox.Lang.Number {{ var n = 12; n = n {op} 3; return n; }}"
        ))
        .unwrap();
        assert_eq!(
            compound.lower().encode().unwrap(),
            expanded.lower().encode().unwrap(),
            "{op}="
        );
        assert_eq!(compound, parse_function_ir(&compound.to_ir_text()).unwrap());
        assert!(
            compound
                .operations()
                .iter()
                .any(|op| matches!(op.operation, Operation::LocalRead { .. }))
        );
        assert!(
            compound
                .operations()
                .iter()
                .any(|op| matches!(op.operation, Operation::LocalAssign { .. }))
        );
    }
    let source = "function f() as Toybox.Lang.Number { var n = 1; n<<=2; return n; }";
    let f = compile_function(source).unwrap();
    let span = f
        .operations()
        .iter()
        .find_map(|op| {
            matches!(op.operation, Operation::LocalAssign { .. }).then_some(op.span.unwrap())
        })
        .unwrap();
    assert_eq!(&source[span.start..span.end], "n<<=2;");
    let f =
        compile_function("function f() as Toybox.Lang.Number { var n = 1; n += 2 * 3; return n; }")
            .unwrap();
    let expanded = compile_function(
        "function f() as Toybox.Lang.Number { var n = 1; n = n + (2 * 3); return n; }",
    )
    .unwrap();
    assert_eq!(
        f.lower().encode().unwrap(),
        expanded.lower().encode().unwrap()
    );
}

#[test]
fn assignments_reuse_scalar_contracts_null_proofs_and_target_scope() {
    for source in [
        "function f() as Toybox.Lang.String { var s = \"x\"; s += 3; s += null; return s; }",
        "function f() as Toybox.Lang.Char { var c = 'A'; c += 1; return c; }",
        "function f() as Toybox.Lang.Number { var n = 1; n += n; return n; }",
        "function f() as Toybox.Lang.Number { var n = 1; n *= (false || true).hashCode(); return n; }",
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); if (n != null) { n += 1; if (n != null) { return n; } } return 0; }",
        "function f() as Toybox.Lang.Number { var n = 1; n /= 0; return n; }",
    ] {
        let f = compile_function(source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    for source in [
        "function f(n as Toybox.Lang.Number) as Toybox.Lang.Number { n += true; return n; }",
        "function f() as Toybox.Lang.Number { missing += 1; return 0; }",
        "function f() as Toybox.Lang.Number { if (true) { var n = 1; } n += 1; return 0; }",
        "function f() as Toybox.Lang.Number { var n = 1; (n) += 1; return n; }",
        "function f() as Toybox.Lang.Number { var n = 1; n += true; return n; }",
        "function f() as Toybox.Lang.Number { var n = 1; n += 'A'; return n; }",
        "function f() as Toybox.Lang.Number { var n = 1; n += \"x\"; return n; }",
        "function f() as Toybox.Lang.Number { var n = 1; n += null; return n; }",
        "function f() as Toybox.Lang.Number { var c = 'A'; c += 'B'; c++; return 0; }",
        "function f() as Toybox.Lang.Number { var c = 'A'; c -= 1; return 0; }",
        "function f() as Toybox.Lang.Number { var s = \"x\"; s *= 2; return 0; }",
        "function f() as Toybox.Lang.Number { var n = true; n |= true; return 0; }",
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); n += 1; return 0; }",
        "function f() as Toybox.Lang.Number { var a = [1]; a[0] += true; return 0; }",
        "function f() as Toybox.Lang.Number { var a = [1]; a += [2]; return 0; }",
        "function f() as Toybox.Lang.Number { var n = 1; n + = 2; return n; }",
        "function f() as Toybox.Lang.Number { var n = 1; n &&= true; return n; }",
        "function f() as Toybox.Lang.Number { var n = 1; return n += 2; }",
        "function f() as Toybox.Lang.Number { var n = 1; n += (n = 2); return n; }",
        "function f() as Toybox.Lang.Number { var n = 1; n >>>= 2; return n; }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
}

#[test]
fn saved_graph_cannot_move_the_read_after_rhs_effects() {
    let f = compile_function(
        "function f() as Toybox.Lang.Number { var n = 1; n += M.value(); return n; }",
    )
    .unwrap();
    let mut ops = f.operations().to_vec();
    let read = ops
        .iter()
        .position(|op| matches!(op.operation, Operation::LocalRead { .. }))
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
}

#[test]
fn staged_rhs_uses_existing_local_budget_and_captured_left_value() {
    let f =
        compile_function("function f() as Toybox.Lang.Number { var n = 3; n += (false || true).hashCode(); return n; }").unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    let ops = f.operations();
    let read = ops
        .iter()
        .position(|op| {
            matches!(
                op.operation,
                Operation::LocalRead {
                    local: ciq_compiler::LocalId(0),
                    ..
                }
            )
        })
        .unwrap();
    let branch = ops
        .iter()
        .position(|op| matches!(op.operation, Operation::Branch { .. }))
        .unwrap();
    assert!(read < branch);
    let params = (0..253)
        .map(|n| format!("p{n}"))
        .collect::<Vec<_>>()
        .join(",");
    compile_function(&format!(
        "function f({params}) as Toybox.Lang.Number {{ var n = 1; n += 1; return n; }}"
    ))
    .unwrap();
    let err = compile_function(&format!(
        "function f({params}) as Toybox.Lang.Number {{ var n = 1; n += (false || true).hashCode(); return n; }}"
    ))
    .unwrap_err();
    assert!(err.message.contains("255-slot limit"), "{}", err.message);
    let nested = |depth: usize| {
        let expr = format!("{}1{}", "M.value(".repeat(depth), ")".repeat(depth));
        format!("function f() as Toybox.Lang.Number {{ var n = 1; n += {expr}; return n; }}")
    };
    compile_function(&nested(64)).unwrap();
    assert!(
        compile_function(&nested(65))
            .unwrap_err()
            .message
            .contains("nesting")
    );
}
