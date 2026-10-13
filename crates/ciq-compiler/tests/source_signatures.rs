use ciq_compiler::{
    ParameterType, ReturnType,
    application::{ConsoleApplication, build_console_application},
    compile_sources, parse_ir, parse_unit_ir,
};

const APP: &str = include_str!("../../../examples/compiler-source-signatures/app.mc");
const VIEW: &str = include_str!("../../../examples/compiler-source-signatures/view.mc");
const VIEWS: &str = include_str!("../../../examples/compiler-source-signatures/views.mc");

#[test]
fn exact_source_contracts_survive_function_calls_and_complete_text_pipelines() {
    let unit = compile_sources(&[APP, VIEWS, VIEW]).unwrap();
    let pass = unit
        .declarations()
        .iter()
        .find(|d| d.function.name() == "pass")
        .unwrap();
    assert_eq!(
        pass.function.parameter_types(),
        &[ParameterType::SourceObject("HelloView".into())]
    );
    assert_eq!(
        pass.function.return_type(),
        ReturnType::SourceObject("HelloView".into())
    );
    let text = unit.to_ir_text();
    assert!(text.contains("parameter-types=source(HelloView)"));
    assert!(text.contains("result=source(HelloView)"));
    let resumed = parse_unit_ir(&text).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [52; 16],
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
    let method = VIEW.replace("function current() { return self.count; }", "function current() { return self.count; } function pass(view as HelloView) as HelloView { return view; }");
    let unit = compile_sources(&["function f() { var view = Views.make(); System.println(view.pass(Views.pass(view)).current()); }", VIEWS, &method]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn source_contracts_do_not_cast_generic_native_or_unrelated_references() {
    let other =
        "class OtherView extends WatchUi.View { function initialize() { View.initialize(); } }";
    for body in [
        "function f() { Views.pass(new Toybox.WatchUi.View()); }",
        "function f() { Views.pass(new OtherView()); }",
        "function f(view as Toybox.WatchUi.View) { Views.pass(view); }",
        "function f() { Views.pass(null); }",
        "function f(view as OtherView) as HelloView { return view; }",
        "function f(view as Toybox.WatchUi.View) as HelloView { return view; }",
        "function f() as HelloView { return new OtherView(); }",
        "function f(view as Missing) { return 0; }",
        "function f() as Missing { return new HelloView(); }",
        "function f(view as MinimalConsoleApp) { return 0; }",
    ] {
        assert!(
            compile_sources(&[body, APP, VIEW, VIEWS, other]).is_err(),
            "{body}"
        );
    }
    for text in [
        "ciq-ir\n.function f parameters=1 parameter-types=source()\n.return-void\n.end\n",
        "ciq-ir\n.function f result=source()\n.new-source %0 HelloView\n.return %0\n.end\n",
        "ciq-ir\n.function f parameters=1 parameter-types=view result=source(HelloView)\n.parameter %0 0\n.return %0\n.end\n",
        "ciq-ir\n.function f\n.call %0 M make result=source()\n.return %0\n.end\n",
    ] {
        assert!(parse_ir(text).is_err(), "{text}");
    }
}

#[test]
fn edited_call_and_signature_claims_are_revalidated_against_definitions() {
    let unit = compile_sources(&[APP, VIEW, VIEWS]).unwrap();
    let text = unit.to_ir_text();
    for changed in [
        text.replace("parameter-types=source(HelloView)", "parameter-types=view"),
        text.replace(
            "Views make result=source(HelloView)",
            "Views make result=view",
        ),
        text.replace("source(HelloView)", "source(Missing)"),
    ] {
        assert_ne!(text, changed);
        assert!(parse_unit_ir(&changed).is_err(), "{changed}");
    }
}
