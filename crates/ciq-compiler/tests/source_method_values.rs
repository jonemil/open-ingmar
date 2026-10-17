use ciq_compiler::{
    Operation,
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, parse_ir, parse_unit_ir,
};

#[test]
fn source_callbacks_and_self_survive_saved_stages_and_application_generation() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-source-methods/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".self %"));
    let resumed = parse_unit_ir(&text).unwrap();
    assert_eq!(unit, resumed);
    let asm = unit.lower();
    assert!(asm.to_text().contains("getself"));
    assert!(asm.to_text().contains("@__ciq_native_method"));
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    let options = ConsoleApplication {
        id: [0x95; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
}

#[test]
fn self_requires_a_receiver_and_stays_an_opaque_object() {
    let text = "ciq-ir\n.function f receiver=instance result=object\n.self %0\n.return %0\n.end\n";
    let f = parse_ir(text).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    assert!(f.lower().to_text().contains("getself"));
    assert!(parse_ir(&text.replace(" receiver=instance", "")).is_err());
    for source in [
        "function f() as Toybox.Lang.Object { return self; }",
        "function f() as Toybox.Lang.Method { return self.method(:f); }",
        "module M { function f() as Toybox.Lang.Object { return self; } }",
        "class V extends Toybox.WatchUi.View { function initialize() { View.initialize(); } function f() { return self + 1; } }",
        "class V extends Toybox.WatchUi.View { function initialize() { View.initialize(); } function f() as Toybox.Lang.Method { return self.method(1); } }",
        "class V extends Toybox.WatchUi.View { function initialize() { View.initialize(); } function f() as Toybox.Lang.Method { return self.method(:f, :f); } }",
        "class V extends Toybox.WatchUi.View { function initialize() { View.initialize(); } function f() { var m=self.method(:f); return m.invoke()+1; } }",
        "class V extends Toybox.WatchUi.View { function initialize() { View.initialize(); } function f() as V { return self; } }",
        "class V extends Toybox.WatchUi.View { function initialize() { View.initialize(); } } function f() as Toybox.Lang.Method { var v=new V(); return v.method(1); }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "accepted {source}");
    }
    assert!(compile("function f() as Toybox.Lang.Object { return self; }").is_err());
}

#[test]
fn declared_source_members_keep_priority_over_reflective_lookup() {
    let unit = compile_sources(&[r#"
class V extends Toybox.WatchUi.View {
    function initialize() { View.initialize(); }
    function method(n as Toybox.Lang.Number) as Toybox.Lang.Number { return n; }
    function f() as Toybox.Lang.Number { return self.method(7); }
}
function f() as Toybox.Lang.Number { var v=new V(); return v.method(8); }
"#])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".call-self "));
    assert!(text.contains(".call-object "));
    assert!(!text.contains(".call-native "));
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    let f = parse_ir(
        "ciq-ir\n.function f receiver=instance result=object\n.self %0\n.return %0\n.end\n",
    )
    .unwrap();
    assert!(matches!(
        f.operations()[0].operation,
        Operation::SelfValue { .. }
    ));
}
