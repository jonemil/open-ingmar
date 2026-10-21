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
fn char_group_preserves_typed_calls_and_complete_pipelines() {
    let context = context(ApiLevel(5, 0, 0));
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-char-methods/app.mc"),
            include_str!("../../../examples/compiler-char-methods/chars.mc"),
        ],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap();
    assert_eq!(unit, resumed);
    assert!(unit.to_ir_text().contains("parameter-types=char"));
    assert!(unit.to_ir_text().contains("result=char"));
    let options = ConsoleApplication {
        id: [28; 16],
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
    let f =
        compile("function f() as Toybox.Lang.Number { return 65.toChar().toNumber(); }").unwrap();
    let symbols = ciq_assembly::Program::parse(
        ".symbol char \"toChar\" id=0x008004c6\n.symbol number \"toNumber\" id=0x008000a7",
    )
    .unwrap();
    assert_eq!(
        ciq_compiler::symbol_bindings(&f.lower(), &symbols).unwrap()["__ciq_native_toChar"],
        0x008004c6
    );
    let entry = compile_sources_with_api(
        &["function start() as Toybox.Lang.Char { return 65.toChar(); }"],
        context,
    )
    .unwrap();
    build_console_application(&entry, &options).unwrap();
}

#[test]
fn char_contracts_are_exact_and_revalidated_in_text() {
    for source in [
        "function f(c as Toybox.Lang.Char) as Toybox.Lang.Char { return c.toLower(); }",
        "function f(c as Toybox.Lang.Char) as Toybox.Lang.Boolean { return c.equals(true); }",
        "function f(c as Toybox.Lang.Char, n as Toybox.Lang.Number or Null) as Toybox.Lang.Boolean { return c.equals(n); }",
        "function f(c as Toybox.Lang.Char) as Toybox.Lang.Number { return c.compareTo(false); }",
    ] {
        let f = compile(source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    for source in [
        "function f() as Toybox.Lang.Number { return 65.toChar(1); }",
        "function f() as Toybox.Lang.Number { return true.toChar(); }",
        "function f() as Toybox.Lang.Number { return 65.toChar(); }",
        "function f() as Toybox.Lang.Number { return 65.toChar() + 1; }",
        "function f() as Toybox.Lang.Number { return 65.toChar() == 65.toChar(); }",
        "function f() as Toybox.Lang.Number { if (65.toChar()) { return 1; } return 0; }",
        "function f() as Toybox.Lang.Number { return 65.toChar().toUpper(1); }",
        "function f() as Toybox.Lang.Number { return 65.toChar().compareTo(null); }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { return n.toChar().toNumber(); }",
        "function f(c as Toybox.Lang.Char or Null) as Toybox.Lang.Number { return c.toNumber(); }",
        "function f() as Toybox.Lang.Char or Null { return true; }",
        "function f() as Toybox.Lang.Number { return 65.toChar().weak(); }",
        "function f() as Toybox.Lang.Number { return 'A'; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let f = compile(
        "function f() as Toybox.Lang.Number { return (65.toChar().toNumber() + 1).abs(); }",
    )
    .unwrap();
    assert!(parse_ir(&f.to_ir_text().replace("toNumber", "toUpper")).is_err());
    let source = "module M { function identity(c as Toybox.Lang.Char) as Toybox.Lang.Char { return c; } function f() as Toybox.Lang.Char { return M.identity(65.toChar()); } }";
    let unit = compile_sources_with_api(&[source], context(ApiLevel(1, 3, 0))).unwrap();
    assert!(
        parse_unit_ir_with_api(
            &unit.to_ir_text().replace("result=char", "result=integer"),
            context(ApiLevel(1, 3, 0))
        )
        .is_err()
    );
    assert!(
        compile_sources_with_api(
            &[&source.replace("M.identity(65.toChar())", "M.identity(65)")],
            context(ApiLevel(1, 3, 0))
        )
        .is_err()
    );
}

#[test]
fn char_class_and_method_introductions_apply_to_source_and_ir() {
    for (source, introduced, previous) in [
        (
            "function f() as Toybox.Lang.Char { return 65.toChar(); }",
            ApiLevel(1, 3, 0),
            ApiLevel(1, 2, 9),
        ),
        (
            "function f(c as Toybox.Lang.Char) as Toybox.Lang.Char { return c; }",
            ApiLevel(1, 3, 0),
            ApiLevel(1, 2, 9),
        ),
        (
            "function f(c as Toybox.Lang.Char) as Toybox.Lang.Number { return c.compareTo(65); }",
            ApiLevel(5, 0, 0),
            ApiLevel(4, 9, 9),
        ),
    ] {
        let text = compile(source).unwrap().to_ir_text();
        for (target, minimum, expected) in [
            (
                previous,
                if introduced == ApiLevel(5, 0, 0) {
                    ApiLevel(1, 3, 0)
                } else {
                    ApiLevel(1, 0, 0)
                },
                "unavailable on target",
            ),
            (introduced, previous, "application minimum API"),
        ] {
            let context = ApiContext::new(target, minimum).unwrap();
            let error = compile_with_api(source, context).unwrap_err();
            assert!(
                error.message.contains(expected),
                "{source}: {}",
                error.message
            );
            assert_eq!(
                error.message,
                parse_ir_with_api(&text, context).unwrap_err().message
            );
        }
        let context = context(introduced);
        assert_eq!(
            compile_with_api(source, context).unwrap(),
            parse_ir_with_api(&text, context).unwrap()
        );
    }
}
