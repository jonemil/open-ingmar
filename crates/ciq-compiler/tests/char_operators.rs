use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources_with_api, compile_with_api, parse_ir, parse_ir_with_api,
    parse_unit_ir_with_api,
};

#[test]
fn char_operators_preserve_result_types_and_complete_pipelines() {
    let context = ApiContext::new(ApiLevel(1, 3, 0), ApiLevel(1, 3, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-char-operators/app.mc"),
            include_str!("../../../examples/compiler-char-operators/chars.mc"),
        ],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [30; 16],
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
    for (source, ty, opcode) in [
        (
            "function f() as Toybox.Lang.Char { return 'A' + 1; }",
            ciq_compiler::ReturnType::Char,
            "addv",
        ),
        (
            "function f() as Toybox.Lang.Char { return 1 + 'A'; }",
            ciq_compiler::ReturnType::Char,
            "addv",
        ),
        (
            "function f() as Toybox.Lang.String { return 'A' + 'B'; }",
            ciq_compiler::ReturnType::String,
            "addv",
        ),
        (
            "function f() as Toybox.Lang.Boolean { return 'A' == 65; }",
            ciq_compiler::ReturnType::Boolean,
            "eq",
        ),
        (
            "function f() as Toybox.Lang.Boolean { return 66 > 'A'; }",
            ciq_compiler::ReturnType::Boolean,
            "gt",
        ),
        (
            "function f(c as Toybox.Lang.Char, n as Toybox.Lang.Number or Null) as Toybox.Lang.Char { if (n == null) { return c; } return c + n; }",
            ciq_compiler::ReturnType::Char,
            "addv",
        ),
    ] {
        let f = compile(source).unwrap();
        assert_eq!(f.return_type(), ty);
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        assert!(f.lower().to_text().contains(opcode));
    }
}

#[test]
fn char_operator_profiles_reject_unverified_pairs_and_edited_consumers() {
    for source in [
        "function f() as Toybox.Lang.Number { return 'B' - 'A'; }",
        "function f() as Toybox.Lang.Number { return 'A' - 1; }",
        "function f() as Toybox.Lang.Number { return 65 - 'A'; }",
        "function f() as Toybox.Lang.Number { return 'A' + 1; }",
        "function f() as Toybox.Lang.Char { return 'A' + 'B'; }",
        "function f() as Toybox.Lang.String { return 'A' + 1; }",
        "function f() as Void { System.println('A' + true); }",
        "function f() as Void { System.println('A' - \"B\"); }",
        "function f() as Void { System.println('A' == true); }",
        "function f() as Void { System.println('A' > true); }",
        "function f() as Void { System.println('A' == null); }",
        "function f(c as Toybox.Lang.Char, n as Toybox.Lang.Number or Null) as Toybox.Lang.Char { return c + n; }",
        "function f() as Toybox.Lang.Char { var c = 'A'; c = c + 'B'; return c; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let f = compile("function f() as Toybox.Lang.Number { return ('A' + 1).toNumber(); }").unwrap();
    let text = f.to_ir_text();
    assert!(text.contains(".integer %1 1"));
    assert!(parse_ir(&text.replace(".integer %1 1", ".boolean %1 true")).is_err());
    assert!(parse_ir(&text.replace(".add", ".subtract")).is_err());
    let f = compile("function f() as Toybox.Lang.Boolean { return 'A' == 65; }").unwrap();
    assert!(parse_ir(&f.to_ir_text().replace(".integer %1 65", ".boolean %1 true")).is_err());
    let source = "function f() as Toybox.Lang.Char { return 'A' + 1; }";
    let context = ApiContext::new(ApiLevel(1, 2, 9), ApiLevel(1, 0, 0)).unwrap();
    let error = compile_with_api(source, context).unwrap_err();
    assert!(error.message.contains("unavailable on target"));
    assert_eq!(
        error.message,
        parse_ir_with_api(&compile(source).unwrap().to_ir_text(), context)
            .unwrap_err()
            .message
    );
}
