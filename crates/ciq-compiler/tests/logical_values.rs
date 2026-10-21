use ciq_compiler::{
    Function, Operation,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, compile_sources_with_api, parse_ir, parse_unit_ir_with_api,
};

#[test]
fn logical_values_preserve_complete_application_and_snapshot_call_order() {
    let context = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-logical-values/app.mc"),
            include_str!("../../../examples/compiler-logical-values/flags.mc"),
        ],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [36; 16],
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
    let f = compile(
        "function f() as Toybox.Lang.Number { return M.first() + (true && false).hashCode(); }",
    )
    .unwrap();
    let operations = f.operations();
    let call = operations
        .iter()
        .position(|op| matches!(op.operation, Operation::ModuleCall { .. }))
        .unwrap();
    assert!(matches!(
        operations[call + 1].operation,
        Operation::LocalInitialize { .. }
    ));
    assert!(f.to_ir_text().find(".call").unwrap() < f.to_ir_text().find(".branch").unwrap());
    assert_eq!(f.lower().to_text().matches("getmv").count(), 1);
}

#[test]
fn logical_values_support_user_self_and_native_argument_contexts() {
    for source in [
        "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.Boolean { return b.equals(true || false); }",
        "function f() as Toybox.Lang.String { return (true && false).toString(); }",
        "function f() as Toybox.Lang.Boolean { return true.equals(false || true); }",
        "function f() as Toybox.Lang.Boolean { return (true || false).equals(false && true); }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Boolean { return n == null || n > 0; }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Boolean { return !(n == null) && n > 0; }",
    ] {
        let f = compile(source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    compile_sources(&["class App extends Application.AppBase {
        function f() as Toybox.Lang.Boolean { return self.take(true || false); }
        function take(b as Toybox.Lang.Boolean) as Toybox.Lang.Boolean { return b; }
        function start() as Void { self.take(true && false); }
    }"])
    .unwrap();
}

#[test]
fn logical_temporaries_recheck_types_initialization_and_do_not_carry_null_proofs() {
    for source in [
        "function f() as Toybox.Lang.Boolean { return false && 1; }",
        "function f() as Toybox.Lang.Boolean { return true || null; }",
        "function f() as Toybox.Lang.Number { var flag = true && false; flag = 1; if(flag){return 1;}return 0; }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { var present = !(n == null) && n > 0; if (present) { return n; } return 0; }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { if ((!(n == null) && true).equals(true)) { return n; } return 0; }",
        "function f() as Toybox.Lang.Number { return -(true && false); }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let f = compile("function f() as Toybox.Lang.Boolean { return true && false; }").unwrap();
    let mut operations = f.operations().to_vec();
    let assigned = operations
        .iter()
        .find_map(|op| match op.operation {
            Operation::LocalAssign { value, .. } => Some(value),
            _ => None,
        })
        .unwrap();
    let definition = operations
        .iter_mut()
        .find(|op| matches!(op.operation, Operation::Boolean { result, .. } if result == assigned))
        .unwrap();
    definition.operation = Operation::Integer {
        result: assigned,
        value: 1,
    };
    assert!(Function::new("bad".into(), operations).is_err());
    let text = f.to_ir_text();
    let init = text
        .lines()
        .find(|line| line.starts_with(".local "))
        .unwrap();
    assert!(parse_ir(&text.replace(init, "")).is_err());
}

#[test]
fn hidden_temporaries_share_the_frame_budget_with_source_locals_and_parameters() {
    let params = (0..254)
        .map(|n| format!("p{n}"))
        .collect::<Vec<_>>()
        .join(",");
    compile(&format!(
        "function f({params}) as Toybox.Lang.Boolean {{ return true && false; }}"
    ))
    .unwrap();
    let error = compile(&format!(
        "function f({params}) as Toybox.Lang.Boolean {{ var flag = true && false; return flag; }}"
    ))
    .unwrap_err();
    assert!(
        error.message.contains("255-slot limit"),
        "{}",
        error.message
    );
}
