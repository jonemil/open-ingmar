use ciq_compiler::{
    FieldKind,
    application::{ConsoleApplication, build_console_application},
    compile_sources, parse_function_ir, parse_unit_ir,
};

const APP: &str = include_str!("../../../examples/compiler-reference-fields/app.mc");
const CONTROLLER: &str = include_str!("../../../examples/compiler-reference-fields/controller.mc");
const VIEW: &str = include_str!("../../../examples/compiler-reference-fields/view.mc");
const VIEWS: &str = include_str!("../../../examples/compiler-reference-fields/views.mc");

#[test]
fn retained_references_and_constructor_arguments_preserve_complete_pipelines() {
    let unit = compile_sources(&[APP, CONTROLLER, VIEW, VIEWS]).unwrap();
    assert_eq!(
        unit.classes()[0].fields[0].kind,
        FieldKind::SourceObject(ciq_compiler::SourceClass {
            name: "ViewController".into(),
            base: ciq_compiler::ClassBase::WatchUiView
        })
    );
    let text = unit.to_ir_text();
    assert!(text.contains(".field-reference target source(HelloView@WatchUi.View)"));
    let resumed = parse_unit_ir(&text).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [53; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    let prg = build_console_application(&unit, &options).unwrap();
    assert_eq!(prg, build_console_application(&resumed, &options).unwrap());
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    let records =
        ciq_prg::data::classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
            .unwrap();
    let slots = records
        .iter()
        .filter_map(|record| match record {
            ciq_prg::data::classes::Record::Class(class) => Some(&class.fields),
            _ => None,
        })
        .flatten()
        .filter(|field| field.kind == 0 && field.value == 0 && field.key & 0xff == 0)
        .count();
    // App's Toybox placeholder plus the controller and its target reference.
    assert_eq!(slots, 3);
}

#[test]
fn reference_initialization_and_assignment_are_checked_before_exposure() {
    let other =
        "class OtherView extends WatchUi.View { function initialize() { View.initialize(); } }";
    for changed in [
        CONTROLLER.replace("self.target = view;", ""),
        CONTROLLER.replace(
            "self.target = view;",
            "System.println(self.target.current()); self.target = view;",
        ),
        CONTROLLER.replace("self.target = view;", "self.current(); self.target = view;"),
        CONTROLLER.replace(
            "self.target = view;",
            "var receiver = self; self.target = view;",
        ),
        CONTROLLER.replace(
            "self.target = view;",
            "self.method(:current); self.target = view;",
        ),
        CONTROLLER.replace(
            "self.target = view;",
            "if (true) { self.target = view; } else { self.target = view; }",
        ),
        CONTROLLER.replace("self.target = view;", "self.target = null;"),
        CONTROLLER.replace("self.target = view;", "System.exit(); self.target = view;"),
        CONTROLLER.replace("self.target = view;", "self.target = new OtherView();"),
        CONTROLLER.replace(
            "self.target = view;",
            "self.target = new Toybox.WatchUi.View();",
        ),
        CONTROLLER.replace("function initialize", "function unused"),
        CONTROLLER.replace("target as HelloView", "target as Missing"),
    ] {
        assert!(
            compile_sources(&[APP, &changed, VIEW, VIEWS, other]).is_err(),
            "{changed}"
        );
    }
    for changed in [
        APP.replace("new ViewController(Views.make())", "new ViewController()"),
        APP.replace("new ViewController(Views.make())", "new ViewController(1)"),
        APP.replace(
            "new ViewController(Views.make())",
            "new ViewController(new OtherView())",
        ),
        APP.replace(
            "function initialize()",
            "function initialize(view as HelloView)",
        ),
    ] {
        assert!(
            compile_sources(&[&changed, CONTROLLER, VIEW, VIEWS, other]).is_err(),
            "{changed}"
        );
    }
    let unit = compile_sources(&[APP, CONTROLLER, VIEW, VIEWS]).unwrap();
    let text = unit.to_ir_text();
    for changed in [
        text.replace(
            ".field-reference target source(HelloView@WatchUi.View)",
            ".field-reference target source(Missing@WatchUi.View)",
        ),
        text.replace(
            ".field-reference target source(HelloView@WatchUi.View)",
            ".field-number target 0",
        ),
        text.replace(
            "target result=source(HelloView@WatchUi.View)",
            "target result=integer",
        ),
        text.lines()
            .filter(|line| !line.starts_with(".assign-self target"))
            .collect::<Vec<_>>()
            .join("\n"),
    ] {
        assert_ne!(text, changed);
        assert!(parse_unit_ir(&changed).is_err(), "{changed}");
    }
    let stops =
        "module Stops { function stop() { System.exit(); } function nested() { Stops.stop(); } }";
    let changed = CONTROLLER.replace("self.target = view;", "Stops.nested(); self.target = view;");
    assert!(compile_sources(&[APP, &changed, VIEW, VIEWS, stops]).is_err());
    let changed = VIEWS.replace(
        "return new HelloView();",
        "System.exit(); return new HelloView();",
    );
    assert!(compile_sources(&[APP, CONTROLLER, VIEW, &changed]).is_err());
}

#[test]
fn constructor_arguments_use_declared_context_and_order_checks() {
    let constructor = CONTROLLER
        .replace(
            "initialize(view as HelloView)",
            "initialize(view as HelloView, amounts as Toybox.Lang.Array<Toybox.Lang.Number>)",
        )
        .replace(
            "self.target = view;",
            "self.target = view; System.println(amounts.size());",
        );
    let unit = compile_sources(&["function f() { var v = new HelloView(); var c = new ViewController(v, []); System.println(c.current()); }", &constructor, VIEW]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let bad = "ciq-ir\n.function f\n.new-source %0 HelloView WatchUi.View\n.local $0 %0\n.read %1 $0\n.call-object %2 %1 HelloView current\n.new-source %3 ViewController WatchUi.View %2\n.return %3\n.end\n";
    // The function carries a constructor declaration obligation; unit checking
    // rejects the Number argument for the declared source reference parameter.
    assert!(parse_function_ir(bad).is_ok());
    let wrong_order = "ciq-ir\n.function f\n.new-source %0 HelloView WatchUi.View\n.local $0 %0\n.read %1 $0\n.call-object %2 %1 HelloView current\n.read %3 $0\n.new-source %4 ViewController WatchUi.View %3 %2\n.return %4\n.end\n";
    assert!(
        parse_function_ir(wrong_order)
            .unwrap_err()
            .message
            .contains("evaluation order")
    );
    for text in [
        "ciq-ir\n.function f\n.new-source\n.return-void\n.end\n",
        "ciq-ir\n.function f receiver=instance\n.read-self\n.return-void\n.end\n",
    ] {
        assert!(parse_function_ir(text).is_err());
    }
}
