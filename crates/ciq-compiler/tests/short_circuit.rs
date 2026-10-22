use ciq_compiler::{
    Function, LocatedOperation, Operation, ParameterType, ReturnType, ValueId,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources_with_api, parse_function_ir, parse_unit_ir_with_api,
};

#[test]
fn short_circuit_complete_application_preserves_all_pipeline_boundaries() {
    let context = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-short-circuit/app.mc"),
            include_str!("../../../examples/compiler-short-circuit/flags.mc"),
        ],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [35; 16],
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
    let f = compile_function("function f(a as Toybox.Lang.Boolean, b as Toybox.Lang.Boolean) as Toybox.Lang.Number { if (a && b) { return 1; } return 0; }").unwrap();
    let branches = f
        .operations()
        .iter()
        .filter(|op| matches!(op.operation, Operation::Branch { .. }))
        .collect::<Vec<_>>();
    assert_eq!(branches.len(), 2);
    assert_eq!(
        branches[0].span.unwrap().end - branches[0].span.unwrap().start,
        1
    );
    assert_eq!(
        branches[1].span.unwrap().end - branches[1].span.unwrap().start,
        1
    );
}

#[test]
fn conditions_validate_every_operand_and_only_propagate_guaranteed_null_facts() {
    for condition in [
        "1 && true",
        "false && 1",
        "true || null",
        "'A' || false",
        "true && \"x\"",
        "true & false",
        "true | false",
    ] {
        assert!(
            compile_function(&format!(
                "function f() as Toybox.Lang.Number {{ if ({condition}) {{ return 1; }} return 0; }}"
            ))
            .is_err(),
            "{condition}"
        );
    }
    for source in [
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { if (!(n == null) || true) { return n; } return 0; }",
        "function f(n as Toybox.Lang.Number or Null, b as Toybox.Lang.Boolean) as Toybox.Lang.Number { if (n == null && b) { return 0; } return n; }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { if (!(n == null) && true) { System.println(n); } return n; }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { var present = !(n == null); if (present) { return n; } return 0; }",
        "function f() as Toybox.Lang.Number { var n = \"8\".toNumber(); while (n == null || 1 > n) { break; } return n; }",
        "function f() as Toybox.Lang.Number { var n = \"8\".toNumber(); if (!(n == null) && n > 0) { n = null; return n; } return 0; }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    for source in [
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { if (!(n == null) && n > 0) { return n; } return 0; }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { if (n == null || n > 0) { return 0; } return n; }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { if (!!(n == null)) { return 0; } return n; }",
        "function f() as Toybox.Lang.Number { var n = \"8\".toNumber(); while (n == null || 1 > n) { n = 2; } return n; }",
    ] {
        let f = compile_function(source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    let f = compile_function("function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { if (!(n == null) && n > 0) { return n; } return 0; }").unwrap();
    let mut operations = f.operations().to_vec();
    let Operation::Branch {
        on_true, on_false, ..
    } = &mut operations
        .iter_mut()
        .find(|op| matches!(op.operation, Operation::Branch { .. }))
        .unwrap()
        .operation
    else {
        unreachable!()
    };
    std::mem::swap(on_true, on_false);
    assert!(
        Function::with_signature(
            "bad".into(),
            vec![ParameterType::NullableInteger],
            ReturnType::Integer,
            operations
        )
        .is_err()
    );
}

#[test]
fn saved_ir_not_guards_recheck_polarity_and_intervening_writes() {
    let f = compile_function(
        "function f() as Toybox.Lang.Number { var n = \"8\".toNumber(); if (n == null) { return 0; } return n; }",
    )
    .unwrap();
    let mut operations = f.operations().to_vec();
    let branch = operations
        .iter()
        .position(|op| matches!(op.operation, Operation::Branch { .. }))
        .unwrap();
    let Operation::Branch {
        condition,
        on_true,
        on_false,
    } = &mut operations[branch].operation
    else {
        unreachable!()
    };
    let operand = *condition;
    *condition = ValueId(1000);
    std::mem::swap(on_true, on_false);
    operations.insert(
        branch,
        LocatedOperation {
            operation: Operation::Not {
                result: ValueId(1000),
                operand,
            },
            span: None,
        },
    );
    let f = Function::new("guard".into(), operations.clone()).unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    let Operation::Branch {
        on_true, on_false, ..
    } = &mut operations[branch + 1].operation
    else {
        unreachable!()
    };
    std::mem::swap(on_true, on_false);
    assert!(Function::new("bad".into(), operations.clone()).is_err());
    // Restore valid polarity, then overwrite the checked local before branching.
    let Operation::Branch {
        on_true, on_false, ..
    } = &mut operations[branch + 1].operation
    else {
        unreachable!()
    };
    std::mem::swap(on_true, on_false);
    let local = operations
        .iter()
        .find_map(|op| match op.operation {
            Operation::LocalInitialize { local, .. } => Some(local),
            _ => None,
        })
        .unwrap();
    operations.splice(
        branch + 1..branch + 1,
        [
            LocatedOperation {
                operation: Operation::Null {
                    result: ValueId(1001),
                },
                span: None,
            },
            LocatedOperation {
                operation: Operation::LocalAssign {
                    local,
                    value: ValueId(1001),
                },
                span: None,
            },
        ],
    );
    assert!(Function::new("bad".into(), operations).is_err());
}

#[test]
fn logical_chain_limits_include_generated_blocks_and_terminators() {
    let source = |n: usize| {
        format!(
            "function f() as Toybox.Lang.Number {{ if ({}) {{ return 1; }} return 0; }}",
            vec!["true"; n].join(" && ")
        )
    };
    compile_function(&source(200)).unwrap();
    let error = compile_function(&source(400)).unwrap_err();
    assert!(
        error.message.contains("1024-operation limit"),
        "{}",
        error.message
    );
}
