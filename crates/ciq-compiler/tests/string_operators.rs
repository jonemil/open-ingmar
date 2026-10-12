use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources_with_api, parse_ir, parse_unit_ir_with_api,
};

#[test]
fn string_operators_preserve_complete_pipelines_and_nullable_results() {
    let context = ApiContext::new(ApiLevel(1, 3, 0), ApiLevel(1, 3, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-string-operators/app.mc"),
            include_str!("../../../examples/compiler-string-operators/strings.mc"),
        ],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [31; 16],
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
    for source in [
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.String { return \"n=\" + n; }",
        "function f(s as Toybox.Lang.String or Null) as Toybox.Lang.String { return s + \"!\"; }",
        "function f(s as Toybox.Lang.String or Null) as Toybox.Lang.Boolean { return \"a\" == s; }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Boolean { return n == \"a\"; }",
        "function f() as Toybox.Lang.String { return false + \"a\"; }",
    ] {
        let f = compile(source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    let early = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    compile_sources_with_api(
        &["function start() { System.println(\"n=\" + 7); System.println(\"a\" == \"a\"); }"],
        early,
    )
    .unwrap();
}

#[test]
fn string_profiles_reject_unsupported_pairs_without_narrowing_nullable_operands() {
    for source in [
        "function f() { return \"a\" + \"b\"; }",
        "function f() as Toybox.Lang.String { return \"a\" - \"b\"; }",
        "function f() as Toybox.Lang.Boolean { return \"a\" > \"b\"; }",
        "function f() as Toybox.Lang.Boolean { return \"true\" == true; }",
        "function f() as Toybox.Lang.Boolean { return false == \"false\"; }",
        "function f(n as Toybox.Lang.Number or Null) { System.println(\"n=\" + n); return n + 1; }",
        "function f(n as Toybox.Lang.Number or Null) { if (\"n\" == n) { return 0; } return n + 1; }",
        "function f(s as Toybox.Lang.String or Null) as Toybox.Lang.String { System.println(s + \"!\"); return s.toUpper(); }",
        "function f(s as Toybox.Lang.String or Null, n as Toybox.Lang.Number or Null) as Toybox.Lang.String { return s + n; }",
        "function f() as Toybox.Lang.String { var s = \"a\"; s = 7; return s; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let dictionary = "class App extends Toybox.Application.AppBase { function onStart(state) { System.println(\"state=\" + state); } }";
    assert!(ciq_compiler::compile_sources(&[dictionary]).is_err());
    let f = compile("function f() as Toybox.Lang.String { return \"a\" + 7; }").unwrap();
    assert!(parse_ir(&f.to_ir_text().replace(".add", ".subtract")).is_err());
    let f = compile("function f() as Toybox.Lang.Boolean { return \"a\" == 7; }").unwrap();
    assert!(parse_ir(&f.to_ir_text().replace(".integer %1 7", ".boolean %1 true")).is_err());
    let source = "module M { function length(n) { return n; } function f() { return M.length(\"a\" + 7); } }";
    assert!(ciq_compiler::compile_sources(&[source]).is_err());
}
