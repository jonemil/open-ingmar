use ciq_compiler::{
    application::{ConsoleApplication, build_console_application},
    compile_sources, parse_ir, parse_unit_ir,
};

const APP: &str = include_str!("../../../examples/compiler-object-calls/app.mc");
const VIEW: &str = include_str!("../../../examples/compiler-object-calls/view.mc");

#[test]
fn constructed_receivers_and_aliases_preserve_complete_text_pipelines() {
    let unit = compile_sources(&[APP, VIEW]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".call-object "));
    assert!(text.contains(".call-object-discard "));
    let resumed = parse_unit_ir(&text).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [51; 16],
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
    for body in [
        "System.println(new HelloView().current());",
        "var v = new HelloView(); var shared = v; if (true) { shared = new HelloView(); } System.println(shared.add(1));",
        "var v = new HelloView(); System.println(v.add(v.current()));",
        "var v = new HelloView(); var n = 0; while (2 > n) { v.bump(); n = n + 1; } System.println(v.current());",
    ] {
        let app = format!("function start() as Void {{ {body} }}");
        let unit = compile_sources(&[&app, VIEW]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
}

#[test]
fn call_contracts_and_local_constructor_classes_are_not_guessed() {
    for body in [
        "var v = new HelloView(); v.missing();",
        "var v = new HelloView(); v.add();",
        "var v = new HelloView(); v.add(\"bad\");",
        "var v = new HelloView(); var invalid = v.bump();",
        "var v = new HelloView(); v.onShow();",
        "var v = Factory.make(); v.current();",
    ] {
        let app = format!("function start() as Void {{ {body} }}");
        let other = "class OtherView extends WatchUi.View { function initialize() { View.initialize(); } } module Factory { function make() as Toybox.WatchUi.View { return new HelloView(); } }";
        assert!(compile_sources(&[&app, VIEW, other]).is_err(), "{body}");
    }
    let unit = compile_sources(&[APP, VIEW]).unwrap();
    let text = unit.to_ir_text();
    for member in ["missing", "onShow"] {
        let changed = text.replace("HelloView current", &format!("HelloView {member}"));
        assert_ne!(text, changed);
        assert!(parse_unit_ir(&changed).is_err());
    }
    let changed = text.replace("HelloView add", "OtherView add");
    assert_ne!(text, changed);
    assert!(parse_unit_ir(&changed).is_err());
    let changed = text.replace("HelloView add", "HelloView add result=string");
    assert_ne!(text, changed);
    assert!(parse_unit_ir(&changed).is_err());
    for text in [
        "ciq-ir\n.function f parameters=1 parameter-types=view\n.parameter %0 0\n.call-object %1 %0 HelloView current\n.return %1\n.end\n",
        "ciq-ir\n.function f\n.new-native %0 Toybox.WatchUi.View\n.call-object %1 %0 HelloView current\n.return %1\n.end\n",
        "ciq-ir\n.function f\n.call-object\n.return-void\n.end\n",
        "ciq-ir\n.function f\n.call-object-discard\n.return-void\n.end\n",
    ] {
        assert!(parse_ir(text).is_err(), "{text}");
    }
}

#[test]
fn receiver_and_arguments_keep_order_and_contextual_array_contracts() {
    let method = VIEW.replace("function current() as Toybox.Lang.Number { return self.count; }", "function current() as Toybox.Lang.Number { return self.count; } function length(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { return a.size(); }");
    let unit = compile_sources(&["function f() as Void { var v = new HelloView(); System.println(v.length([])); System.println(v.length([v.current()])); }", &method]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let bad = "ciq-ir\n.function f\n.new-source %0 HelloView WatchUi.View\n.local $0 %0\n.read %1 $0\n.integer %2 1\n.call-object %3 %1 HelloView add %2\n.read %4 $0\n.call-object %5 %4 HelloView add %3\n.return %5\n.end\n";
    assert!(
        parse_ir(bad)
            .unwrap_err()
            .message
            .contains("evaluation order")
    );
}
