use ciq_compiler::{
    Operation,
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources, parse_function_ir, parse_unit_ir,
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
    let f = parse_function_ir(text).unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    assert!(f.lower().to_text().contains("getself"));
    assert!(parse_function_ir(&text.replace(" receiver=instance", "")).is_err());
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
    assert!(compile_function("function f() as Toybox.Lang.Object { return self; }").is_err());
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
    let f = parse_function_ir(
        "ciq-ir\n.function f receiver=instance result=object\n.self %0\n.return %0\n.end\n",
    )
    .unwrap();
    assert!(matches!(
        f.operations()[0].operation,
        Operation::SelfValue { .. }
    ));
}

#[test]
fn implicit_method_lookup_reuses_explicit_self_calls() {
    for base in [
        "Toybox.Lang.Object",
        "Toybox.WatchUi.View",
        "Toybox.Application.AppBase",
    ] {
        for body in [
            "function bind(s as Toybox.Lang.Symbol) as Toybox.Lang.Method { return method(s); }",
            "function bind() { method(:bind); }",
            "function bind(b as Toybox.Lang.Boolean) as Toybox.Lang.Method { return method(b ? :bind : :bind); }",
            "function method(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { return a.size(); } function bind() as Toybox.Lang.Number { return method([1, 2]); }",
            "function method(n as Toybox.Lang.Number) as Toybox.Lang.Number { return n; } function bind() as Toybox.Lang.Number { return method(7); }",
            "function method(n as Toybox.Lang.Number) as Void {} function bind() { method(7); }",
        ] {
            let implicit = format!("class C extends {base} {{ {body} }}");
            let explicit = implicit
                .replace("return method(", "return self.method(")
                .replace("{ method(", "{ self.method(");
            let a = compile_sources(&[&implicit]).unwrap();
            let b = compile_sources(&[&explicit]).unwrap();
            let without_spans = |text: String| {
                text.lines()
                    .map(|line| line.split(" span=").next().unwrap())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            assert_eq!(
                without_spans(a.to_ir_text()),
                without_spans(b.to_ir_text()),
                "{implicit}"
            );
            assert_eq!(a.lower(), b.lower());
            assert_eq!(a, parse_unit_ir(&a.to_ir_text()).unwrap());
        }
    }
}

#[test]
fn implicit_method_lookup_rejects_non_instance_and_shadowed_calls() {
    for source in [
        "function f() as Toybox.Lang.Method { return method(:f); }",
        "module M { function f() { method(:f); } }",
        "class C extends Toybox.Lang.Object { function f(method as Toybox.Lang.Number) { method(:f); } }",
        "class C extends Toybox.Lang.Object { function f() { var method = 1; method(:f); } }",
        "class C extends Toybox.Lang.Object { var method = 1; function f() { method(:f); } }",
        "class C extends Toybox.Lang.Object { function f() { method(1); } }",
        "class C extends Toybox.Lang.Object { function f() { method(:f, :f); } }",
        "class C extends Toybox.Lang.Object { function f() { other(:f); } }",
        "class C extends Toybox.Lang.Object { var required as Toybox.Lang.Symbol; function initialize() { method(:initialize); self.required = :ready; } }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "accepted {source}");
    }
    let explicit = "class C extends Toybox.Lang.Object { function bind(method as Toybox.Lang.Number) as Toybox.Lang.Method { return self.method(:bind); } }";
    compile_sources(&[explicit]).unwrap();
}

#[test]
fn implicit_method_calls_preserve_nesting_limits() {
    for (depth, accepted) in [(64, true), (65, false)] {
        let expression = format!("{}1{}", "method(".repeat(depth), ")".repeat(depth));
        let source = format!(
            "class C extends Toybox.Lang.Object {{ function method(n as Toybox.Lang.Number) as Toybox.Lang.Number {{ return n; }} function f() as Toybox.Lang.Number {{ return {expression}; }} }}"
        );
        let result = compile_sources(&[&source]);
        if accepted {
            result.unwrap();
        } else {
            assert!(result.unwrap_err().diagnostic.message.contains("nesting"));
        }
    }
}
