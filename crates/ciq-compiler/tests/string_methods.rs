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
fn string_group_preserves_complete_app_and_symbolic_pipelines() {
    let context = context(ApiLevel(5, 0, 0));
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-string-methods/app.mc"),
            include_str!("../../../examples/compiler-string-methods/texts.mc"),
        ],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [25; 16],
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
        ("return \"a\".compareTo(\"b\");", "compareTo", 0x00800e50),
        ("return \"a\".hashCode();", "hashCode", 0x008000ad),
        ("System.println(\"a\".equals(null));", "equals", 0x008000b3),
        (
            "var n = \"FF\".toNumberWithBase(16); if (n == null) { return 0; } return n;",
            "toNumberWithBase",
            0x008004c3,
        ),
    ] {
        let f = compile(&format!("function f() {{ {body} }}")).unwrap();
        let symbols = ciq_assembly::Program::parse(&format!(".symbol method \"{name}\" id=0x{id:08x}\n.symbol system \"Toybox_System\" id=0x00800005\n.symbol print \"println\" id=0x00800067")).unwrap();
        let bindings = ciq_compiler::symbol_bindings(&f.lower(), &symbols).unwrap();
        assert_eq!(bindings[&format!("__ciq_native_{name}")], id);
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
}

#[test]
fn string_argument_profiles_and_nullable_results_share_ir_validation() {
    for source in [
        "function f(x as Toybox.Lang.String or Null) as Toybox.Lang.Number { if (x == null) { return 0; } return \"a\".compareTo(x); }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { if (n == null) { return 0; } var value = \"10\".toNumberWithBase(n); if (value == null) { return 0; } return value; }",
        "function f(x as Toybox.Lang.String or Null) as Toybox.Lang.Boolean { return \"a\".equals(x); }",
        "function f(x as Toybox.Lang.Number or Null) as Toybox.Lang.Boolean { return \"a\".equals(x); }",
        "function f() as Toybox.Lang.Boolean { return \"a\".equals(true); }",
        "function f() as Toybox.Lang.Number or Null { return \"FF\".toNumberWithBase(16); }",
    ] {
        let f = compile(source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    for source in [
        "function f() as Toybox.Lang.Number { return \"a\".compareTo(null); }",
        "function f() as Toybox.Lang.Number { return \"a\".compareTo(); }",
        "function f(x as Toybox.Lang.String or Null) as Toybox.Lang.Number { return \"a\".compareTo(x); }",
        "function f(x as Toybox.Lang.String or Null) as Toybox.Lang.Number { return x.hashCode(); }",
        "function f() as Toybox.Lang.Number { return \"a\".hashCode(1); }",
        "function f() as Toybox.Lang.Number { return \"a\".equals(); }",
        "function f() as Void { var n = \"10\".toNumberWithBase(); }",
        "function f() as Void { var n = \"10\".toNumberWithBase(\"2\"); }",
        "function f() as Void { var n = \"10\".toNumberWithBase(true); }",
        "function f() as Void { var n = \"10\".toNumberWithBase(null); }",
        "function f(n as Toybox.Lang.Number or Null) as Void { var value = \"10\".toNumberWithBase(n); }",
        "function f() as Toybox.Lang.Number { return \"10\".toNumberWithBase(2); }",
        "function f() as Void { System.println(\"10\".toNumberWithBase(2)); }",
        "function f() as Toybox.Lang.Number { return \"a\".equals(\"b\") + 1; }",
        "function f() as Void { var n = 7.toNumberWithBase(2); }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let f =
        compile("function f() as Toybox.Lang.Number { return \"a\".compareTo(\"b\"); }").unwrap();
    let text = f.to_ir_text();
    assert!(text.contains(".string %1 \"b\""));
    assert!(parse_ir(&text.replace(".string %1 \"b\"", ".null %1")).is_err());
    assert!(parse_ir(&text.replace("compareTo", "toNumberWithBase")).is_err());
    let f = compile("function f() as Toybox.Lang.Number { var n = \"10\".toNumberWithBase(2); if (n == null) { return 0; } return n; }").unwrap();
    let mut operations = f.operations().to_vec();
    for op in &mut operations {
        if let ciq_compiler::Operation::Branch {
            on_true, on_false, ..
        } = &mut op.operation
        {
            std::mem::swap(on_true, on_false);
        }
    }
    assert!(ciq_compiler::Function::new("bad".into(), operations).is_err());
}

#[test]
fn string_method_introductions_are_checked_in_source_and_text() {
    for (source, introduced, previous) in [
        (
            "function f() as Void { var n = \"FF\".toNumberWithBase(16); }",
            ApiLevel(1, 4, 1),
            ApiLevel(1, 4, 0),
        ),
        (
            "function f() as Toybox.Lang.Number { return \"a\".compareTo(\"b\"); }",
            ApiLevel(5, 0, 0),
            ApiLevel(4, 9, 9),
        ),
    ] {
        let text = compile(source).unwrap().to_ir_text();
        for (target, minimum, expected) in [
            (previous, ApiLevel(1, 0, 0), "unavailable on target"),
            (introduced, previous, "application minimum API"),
        ] {
            let context = ApiContext::new(target, minimum).unwrap();
            let source_error = compile_with_api(source, context).unwrap_err();
            let text_error = parse_ir_with_api(&text, context).unwrap_err();
            assert!(source_error.message.contains(expected));
            assert_eq!(source_error.message, text_error.message);
        }
        assert_eq!(
            compile_with_api(source, context(introduced)).unwrap(),
            parse_ir_with_api(&text, context(introduced)).unwrap()
        );
    }
    for source in [
        "function f() as Toybox.Lang.Number { return \"a\".hashCode(); }",
        "function f() as Toybox.Lang.Boolean { return \"a\".equals(null); }",
    ] {
        compile_with_api(source, context(ApiLevel(1, 0, 0))).unwrap();
    }
}
