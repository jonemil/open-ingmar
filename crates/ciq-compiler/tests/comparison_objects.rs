use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, compile_sources_with_api, compile_with_api, parse_ir,
    parse_ir_with_api, parse_unit_ir,
};

#[test]
fn comparison_object_arguments_preserve_complete_saved_stages() {
    let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[include_str!(
            "../../../examples/compiler-comparison-objects/app.mc"
        )],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [97; 16],
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
}

#[test]
fn compare_to_uses_the_declared_object_domain_in_source_and_ir() {
    for receiver in ["2", "2l", "2.0", "2.0d", "true", "'B'", "\"b\""] {
        for argument in [
            "other", "1", "1l", "1.0", "1.0d", "true", "'A'", "\"a\"", ":alpha", "[1]", "{1=>2}",
            "[1]b",
        ] {
            let source = format!(
                "function f(other as Lang.Object) {{return {receiver}.compareTo({argument});}}"
            );
            let f = compile(&source).unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
            let asm = f.lower().to_text();
            assert!(
                asm.contains("compareTo") && asm.contains("invokem"),
                "{asm}"
            );
        }
    }
    for alias in ["Numeric", "Integer", "Decimal"] {
        for body in ["return a.compareTo(b);", "return 2.compareTo(b);"] {
            let source = format!("function f(a as Lang.{alias},b as Lang.{alias}) {{{body}}}");
            let f = compile(&source).unwrap();
            assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        }
    }
    for source in [
        "function f(o as Lang.Object or Null) {if(o==null){return 0;}return 2.compareTo(o);}",
        "function f(o as Lang.Object or Null) {if(o==null){return 0;}return \"a\".compareTo(o);}",
        "function f(o as Lang.Numeric or Null) {return o!=null ? 2.compareTo(o) : 0;}",
    ] {
        let f = compile(source).unwrap();
        let text = f.to_ir_text();
        assert_eq!(f, parse_ir(&text).unwrap());
        if text.contains(".not-equal ") {
            assert!(parse_ir(&text.replace(".not-equal ", ".equal ")).is_err());
        }
    }
}

#[test]
fn equals_accepts_supported_object_classes_interfaces_and_null() {
    for receiver in [
        "2", "2l", "2.0", "2.0d", "true", "'B'", "\"b\"", "[1]", "{1=>2}", "[1]b",
    ] {
        for ty in [
            "Lang.Object",
            "Lang.Object or Null",
            "Toybox.WatchUi.View",
            "Toybox.WatchUi.BehaviorDelegate",
            "Toybox.Graphics.Dc",
            "Lang.Comparable",
            "Lang.Comparator",
            "Lang.Numeric or Null",
        ] {
            let source =
                format!("function f(o as {ty}) as Lang.Boolean {{return {receiver}.equals(o);}}");
            let f = compile(&source).unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        }
    }
    let source = "class C {function value(){return 1;}}module M {function f(c as C) as Lang.Boolean {return 2.equals(c);}}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn object_comparison_checks_keep_null_arity_result_and_api_requirements() {
    for source in [
        "function f(){return 2.compareTo(null);}",
        "function f(o as Lang.Object or Null){return 2.compareTo(o);}",
        "function f(o as Lang.Object or Null){return \"a\".compareTo(o);}",
        "function f(o as Lang.Numeric or Null){return 2.compareTo(o);}",
        "function f(o as Lang.Object or Null){var ok=o!=null;if(ok){return 2.compareTo(o);}return 0;}",
        "function f(){return 2.compareTo();}",
        "function f(o as Lang.Object){return 2.compareTo(o,o);}",
        "function f(o as Lang.Object){return 2.equals(o)+1;}",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let source = "function f(o as Lang.Object){return 2.compareTo(o);}";
    let f = compile(source).unwrap();
    assert!(
        parse_ir(
            &f.to_ir_text()
                .replace("parameter-types=object", "parameter-types=nullable-object")
        )
        .is_err()
    );
    let old = ApiContext::new(ApiLevel(4, 2, 0), ApiLevel(4, 2, 0)).unwrap();
    assert!(compile_with_api(source, old).is_err());
    assert!(parse_ir_with_api(&f.to_ir_text(), old).is_err());
}
