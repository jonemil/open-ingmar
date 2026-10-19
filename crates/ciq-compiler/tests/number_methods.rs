use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources_with_api, compile_with_api, parse_ir, parse_ir_with_api,
    parse_unit_ir_with_api,
};

fn context(target: ApiLevel, minimum: ApiLevel) -> ApiContext {
    ApiContext::new(target, minimum).unwrap()
}

#[test]
fn number_group_preserves_complete_app_and_text_pipelines() {
    let context = context(ApiLevel(5, 0, 0), ApiLevel(5, 0, 0));
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-number-methods/app.mc"),
            include_str!("../../../examples/compiler-number-methods/numbers.mc"),
        ],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [24; 16],
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
    for (expression, name, id) in [
        ("7.toNumber()", "toNumber", 0x008000a7),
        ("7.hashCode()", "hashCode", 0x008000ad),
        ("7.equals(null)", "equals", 0x008000b3),
        ("7.compareTo(3)", "compareTo", 0x00800e50),
    ] {
        let f = compile(&format!("function f() {{ System.println({expression}); }}")).unwrap();
        let symbols = ciq_assembly::Program::parse(&format!(".symbol method \"{name}\" id=0x{id:08x}\n.symbol system \"Toybox_System\" id=0x00800005\n.symbol print \"println\" id=0x00800067")).unwrap();
        let bindings = ciq_compiler::symbol_bindings(&f.lower(), &symbols).unwrap();
        assert_eq!(bindings[&format!("__ciq_native_{name}")], id);
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
}

#[test]
fn object_arguments_widen_only_through_declared_inheritance() {
    for source in [
        "function f() as Toybox.Lang.Boolean { return 7.equals(7); }",
        "function f() as Toybox.Lang.Boolean { return 7.equals(true); }",
        "function f() as Toybox.Lang.Boolean { return 7.equals(\"7\"); }",
        "function f() as Toybox.Lang.Boolean { return 7.equals(null); }",
        "function f(x as Toybox.Lang.Number or Null) as Toybox.Lang.Boolean { return 7.equals(x); }",
        "function f(x as Toybox.Lang.String or Null) as Toybox.Lang.Boolean { return 7.equals(x); }",
        "function f(x as Toybox.Lang.Number or Null) { if (x == null) { return 0; } return 7.compareTo(x); }",
        "function f() { return 7.compareTo(true); }",
    ] {
        let f = compile(source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    for source in [
        "function f() { return 7.equals(); }",
        "function f() { return 7.hashCode(1); }",
        "function f() { return 7.toNumber(1); }",
        "function f() { return 7.compareTo(); }",
        "function f() { return 7.compareTo(1, 2); }",
        "function f() { return 7.compareTo(null); }",
        "function f(x as Toybox.Lang.Number or Null) { return 7.compareTo(x); }",
        "function f(x as Toybox.Lang.Number or Null) { return x.hashCode(); }",
        "function f() { return 7.equals(7) + 1; }",
        "function f() { return true.abs(); }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let wrong_user_type =
        "module M { function number(n) { return n; } function f() { return M.number(true); } }";
    assert!(
        compile_sources_with_api(
            &[wrong_user_type],
            context(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0))
        )
        .is_err()
    );
    let f = compile("function f() { return 7.compareTo(3); }").unwrap();
    let mut operations = f.operations().to_vec();
    let op = operations
        .iter_mut()
        .find(|op| {
            matches!(
                op.operation,
                ciq_compiler::Operation::Integer { value: 3, .. }
            )
        })
        .unwrap();
    let ciq_compiler::Operation::Integer { result, .. } = op.operation else {
        unreachable!()
    };
    op.operation = ciq_compiler::Operation::Null { result };
    assert!(ciq_compiler::Function::new("bad".into(), operations).is_err());
}

#[test]
fn compare_introduction_is_checked_in_source_and_saved_ir() {
    let source = "function f() { return 7.compareTo(3); }";
    let text = compile(source).unwrap().to_ir_text();
    for (target, minimum, expected) in [
        (
            ApiLevel(4, 9, 9),
            ApiLevel(1, 0, 0),
            "unavailable on target",
        ),
        (
            ApiLevel(5, 0, 0),
            ApiLevel(4, 9, 9),
            "application minimum API",
        ),
    ] {
        let context = context(target, minimum);
        let source_error = compile_with_api(source, context).unwrap_err();
        let text_error = parse_ir_with_api(&text, context).unwrap_err();
        assert!(source_error.message.contains(expected));
        assert_eq!(source_error.message, text_error.message);
    }
    let context = context(ApiLevel(5, 0, 0), ApiLevel(5, 0, 0));
    assert_eq!(
        compile_with_api(source, context).unwrap(),
        parse_ir_with_api(&text, context).unwrap()
    );
    for expression in ["7.toNumber()", "7.hashCode()", "7.equals(null)"] {
        let source = format!("function f() {{ System.println({expression}); }}");
        compile_with_api(
            &source,
            ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap(),
        )
        .unwrap();
    }
}
