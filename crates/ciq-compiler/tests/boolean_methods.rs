use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources_with_api, compile_with_api, parse_ir, parse_ir_with_api,
    parse_unit_ir_with_api,
};

fn context(level: ApiLevel) -> ApiContext {
    ApiContext::new(level, level).unwrap()
}

#[test]
fn boolean_group_preserves_complete_app_and_symbolic_pipelines() {
    let context = context(ApiLevel(5, 0, 0));
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-boolean-methods/app.mc"),
            include_str!("../../../examples/compiler-boolean-methods/flags.mc"),
        ],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [26; 16],
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
    for (body, name, id) in [
        ("return true.compareTo(false);", "compareTo", 0x00800e50),
        ("return true.hashCode();", "hashCode", 0x008000ad),
        ("System.println(true.equals(null));", "equals", 0x008000b3),
        ("System.println(false.toString());", "toString", 0x008000af),
    ] {
        let f = compile(&format!("function f() {{ {body} }}")).unwrap();
        let symbols = ciq_assembly::Program::parse(&format!(".symbol method \"{name}\" id=0x{id:08x}\n.symbol system \"Toybox_System\" id=0x00800005\n.symbol print \"println\" id=0x00800067")).unwrap();
        let bindings = ciq_compiler::symbol_bindings(&f.lower(), &symbols).unwrap();
        assert_eq!(bindings[&format!("__ciq_native_{name}")], id);
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
}

#[test]
fn boolean_argument_and_result_contracts_are_rechecked_in_ir() {
    for source in [
        "function f(x as Toybox.Lang.Boolean) { return x.compareTo(1); }",
        "function f(n as Toybox.Lang.Number or Null) { if (n == null) { return 0; } return true.compareTo(n); }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Boolean { return true.equals(n); }",
        "function f(s as Toybox.Lang.String or Null) as Toybox.Lang.Boolean { return false.equals(s); }",
        "function f() as Toybox.Lang.String { return true.toString(); }",
        "function f() as Toybox.Lang.Boolean { return true.equals(false); }",
    ] {
        let f = compile(source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    for source in [
        "function f() { return true.compareTo(null); }",
        "function f() { return true.compareTo(); }",
        "function f() { return true.compareTo(0, 1); }",
        "function f(n as Toybox.Lang.Number or Null) { return true.compareTo(n); }",
        "function f() { return true.hashCode(1); }",
        "function f() { return true.equals(); }",
        "function f() { System.println(true.toString(1)); }",
        "function f() { return true.toNumber(); }",
        "function f() { return true.equals(false) + 1; }",
        "function f() { return true.toString(); }",
        "function f() { if (true.compareTo(false)) { return 1; } return 0; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let f = compile("function f() { return true.compareTo(false); }").unwrap();
    let text = f.to_ir_text();
    assert!(text.contains(".boolean %1 false"));
    assert!(parse_ir(&text.replace(".boolean %1 false", ".null %1")).is_err());
    let arithmetic = compile("function f() { return true.compareTo(false) + 1; }").unwrap();
    assert!(parse_ir(&arithmetic.to_ir_text().replace("compareTo", "equals")).is_err());
    let f = compile("function f(n as Toybox.Lang.Number or Null) { if (n == null) { return 0; } return true.compareTo(n); }").unwrap();
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
            operations,
        )
        .is_err()
    );
}

#[test]
fn boolean_method_availability_matches_source_and_ir() {
    let source = "function f() { return true.compareTo(false); }";
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
        let context = ApiContext::new(target, minimum).unwrap();
        let source_error = compile_with_api(source, context).unwrap_err();
        let text_error = parse_ir_with_api(&text, context).unwrap_err();
        assert!(source_error.message.contains(expected));
        assert_eq!(source_error.message, text_error.message);
    }
    let context = context(ApiLevel(5, 0, 0));
    assert_eq!(
        compile_with_api(source, context).unwrap(),
        parse_ir_with_api(&text, context).unwrap()
    );
    for source in [
        "function f() { return false.hashCode(); }",
        "function f() as Toybox.Lang.Boolean { return true.equals(null); }",
        "function f() as Toybox.Lang.String { return false.toString(); }",
    ] {
        let context = self::context(ApiLevel(1, 0, 0));
        let f = compile_with_api(source, context).unwrap();
        assert_eq!(f, parse_ir_with_api(&f.to_ir_text(), context).unwrap());
    }
}
