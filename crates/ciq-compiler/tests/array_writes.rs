use ciq_compiler::{
    Function, Operation, ValueId,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources_with_api, parse_function_ir, parse_function_ir_with_api,
    parse_unit_ir_with_api,
};

#[test]
fn writes_preserve_complete_source_ir_assembly_and_prg_pipelines() {
    let ctx = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(2, 1, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-array-writes/app.mc"),
            include_str!("../../../examples/compiler-array-writes/writes.mc"),
        ],
        ctx,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), ctx).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [42; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
    let assembly = unit.lower();
    let reparsed = ciq_assembly::Program::parse(&assembly.to_text()).unwrap();
    assert_eq!(assembly, reparsed);
}

#[test]
fn write_consumes_three_operands_without_result_or_hidden_slots() {
    let source = "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>, i as Toybox.Lang.Number, v as Toybox.Lang.Number) as Void { a[i] = v; }";
    let f = compile_function(source).unwrap();
    assert_eq!(
        f.lower().encode().unwrap(),
        [0x35, 4, 0x12, 1, 0x12, 2, 0x12, 3, 0x11, 0x16]
    );
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    let span = f
        .operations()
        .iter()
        .find_map(|op| {
            matches!(op.operation, Operation::ArrayWrite { .. }).then_some(op.span.unwrap())
        })
        .unwrap();
    assert_eq!(&source[span.start..span.end], "a[i] = v;");
    let ctx = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    assert_eq!(f, parse_function_ir_with_api(&f.to_ir_text(), ctx).unwrap());
    for index in ["-1", "2", "2147483647"] {
        compile_function(&format!(
            "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Void {{ a[{index}] = 9; }}"
        ))
        .unwrap();
    }
}

#[test]
fn writes_reject_wrong_types_unproven_nulls_and_nonassignment_targets() {
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { a[0] = 'X'; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { a[0] = true; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { a[0] = null; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { a[0] = [2]; return 0; }",
        "function f() as Toybox.Lang.Number { var a = [1]; a['X'] = 2; return 0; }",
        "function f() as Toybox.Lang.Number { var a = [1]; a[true] = 2; return 0; }",
        "function f() as Toybox.Lang.Number { var a = [1]; a[null] = 2; return 0; }",
        "function f() as Toybox.Lang.Number { var a = [1]; a[0][0] = 2; return 0; }",
        "function f() as Toybox.Lang.Number { var a = [1]; return a[0] = 2; }",
        "function f() as Toybox.Lang.Number { var a = [1]; a[0]; return 0; }",
        "function f() as Toybox.Lang.Number { var a = [1]; (a) = [2]; return 0; }",
        "function f() as Toybox.Lang.Number { 1[0] = 2; return 0; }",
        "function f() as Toybox.Lang.Number { \"ab\"[0] = 2; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Toybox.Lang.Number { a[0] = 2; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.WatchUi.View>, v as Toybox.WatchUi.View) as Toybox.Lang.Number { a[0] = null; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>, n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { a[n] = 2; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>, n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { a[0] = n; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>, n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { if (n != null) { n = null; a[0] = n; } return 0; }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    let f = compile_function("function f(a as Toybox.Lang.Array<Toybox.Lang.Number>, n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { if (n != null) { a[n] = n; } return 0; }").unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
}

#[test]
fn saved_ir_rechecks_operands_and_effect_order() {
    let f = compile_function(
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>, i as Toybox.Lang.Number, v as Toybox.Lang.Number) as Void { a[i] = v; }",
    )
    .unwrap();
    let text = f.to_ir_text();
    for (from, to) in [
        ("number-array", "char-array"),
        (".array-write %0 %1 %2", ".array-write %0 %1"),
        (".array-write %0 %1 %2", ".array-write %0 %1 %2 %3"),
        (".array-write %0 %1 %2", ".array-write %0 %1 %999"),
        (".array-write %0 %1 %2", ".array-write %0 %1 %1"),
    ] {
        assert!(text.contains(from), "missing fixture: {from}\n{text}");
        assert!(parse_function_ir(&text.replace(from, to)).is_err(), "{to}");
    }
    let f = compile_function("function f(a as Toybox.Lang.Array<Toybox.Lang.Number>, b as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { a[0] = b[0]; return 0; }").unwrap();
    let mut ops = f.operations().to_vec();
    let read = ops
        .iter()
        .position(|op| matches!(op.operation, Operation::ArrayRead { .. }))
        .unwrap();
    let moved = ops.drain(read - 2..=read).collect::<Vec<_>>();
    ops.splice(0..0, moved);
    let err = Function::with_signature(
        "bad".into(),
        f.parameter_types().to_vec(),
        f.return_type(),
        ops,
    )
    .unwrap_err();
    assert!(err.message.contains("evaluation order"), "{}", err.message);
    let mut ops = f.operations().to_vec();
    for op in &mut ops {
        if let Operation::ArrayWrite { value, .. } = &mut op.operation {
            *value = ValueId(999);
        }
    }
    assert!(
        Function::with_signature(
            "bad".into(),
            f.parameter_types().to_vec(),
            f.return_type(),
            ops
        )
        .is_err()
    );
}

#[test]
fn logical_operands_snapshot_receiver_index_and_value_in_order() {
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { a[0] = (false || true).hashCode(); return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { a[(true && false).hashCode()] = 2; return 0; }",
        "function f() as Toybox.Lang.Number { [1, 2][0] = 3; return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { var i = 0; while (i < a.size()) { a[i] = i; i = i + 1; } return 0; }",
    ] {
        let f = compile_function(source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
}

#[test]
fn staged_write_temporaries_use_the_existing_frame_budget() {
    let params = std::iter::once("a as Toybox.Lang.Array<Toybox.Lang.Number>".into())
        .chain((1..254).map(|n| format!("p{n}")))
        .collect::<Vec<String>>()
        .join(",");
    compile_function(&format!("function f({params}) as Void {{ a[0] = 9; }}")).unwrap();
    let error = compile_function(&format!(
        "function f({params}) as Void {{ a[0] = (false || true).hashCode(); }}"
    ))
    .unwrap_err();
    assert!(
        error.message.contains("255-slot limit"),
        "{}",
        error.message
    );
}

#[test]
fn char_and_view_writes_preserve_unit_and_complete_application_pipelines() {
    let ctx = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(2, 1, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-typed-array-writes/app.mc"),
            include_str!("../../../examples/compiler-typed-array-writes/writes.mc"),
        ],
        ctx,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), ctx).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [54; 16],
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
fn element_writes_share_encoding_but_recheck_invariant_types_and_api_levels() {
    for element in ["Toybox.Lang.Char", "Toybox.WatchUi.View"] {
        let f = compile_function(&format!("function f(a as Toybox.Lang.Array<{element}>, i as Toybox.Lang.Number, v as {element}) as Void {{ a[i] = v; }}")).unwrap();
        assert_eq!(
            f.lower().encode().unwrap(),
            [0x35, 4, 0x12, 1, 0x12, 2, 0x12, 3, 0x11, 0x16]
        );
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        let introduction = if element.ends_with("Char") {
            ApiLevel(1, 3, 0)
        } else {
            ApiLevel(1, 0, 0)
        };
        let ctx = ApiContext::new(introduction, introduction).unwrap();
        assert_eq!(f, parse_function_ir_with_api(&f.to_ir_text(), ctx).unwrap());
        if element.ends_with("Char") {
            let older = ApiContext::new(ApiLevel(1, 2, 0), ApiLevel(1, 2, 0)).unwrap();
            assert!(parse_function_ir_with_api(&f.to_ir_text(), older).is_err());
        }
    }
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Void { a[0] = null; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Void { a[0] = \"x\"; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Void { a['a'] = 'b'; }",
        "function f(a as Toybox.Lang.Array<Toybox.WatchUi.View>, d as Toybox.WatchUi.BehaviorDelegate) as Void { a[0] = d; }",
        "function f(a as Toybox.Lang.Array<Toybox.WatchUi.View>, d as Toybox.Graphics.Dc) as Void { a[0] = d; }",
        "function f(a as Toybox.Lang.Array<Toybox.WatchUi.View>) as Void { a[0] = 1; }",
        "function f(a as Toybox.Lang.Array<Toybox.WatchUi.View>) as Void { a[0] = ['a']; }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    let f = compile_function("function f(a as Toybox.Lang.Array<Toybox.WatchUi.View>, v as Toybox.WatchUi.View) as Void { a[0] = v; }").unwrap();
    for (from, to) in [
        ("view-array", "char-array"),
        ("view-array", "number-array"),
        ("view-array", "tuple-view"),
        (
            "types=view-array:view",
            "types=view-array:behavior-delegate",
        ),
    ] {
        let text = f.to_ir_text();
        assert!(text.contains(from), "fixture: {from}\n{text}");
        assert!(parse_function_ir(&text.replace(from, to)).is_err(), "{to}");
    }
}
