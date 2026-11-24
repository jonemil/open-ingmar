use ciq_compiler::{application::ConsoleApplication, compile_sources, parse_unit_ir};

#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn nested_construction_survives_the_saved_application_pipeline() {
    let unit = checking::compile_sources(&[include_str!(
        "../../../examples/compiler-nested-classes/app.mc"
    )])
    .unwrap();
    assert!(!unit.modules().contains("Scope.Outer"));
    common::application(
        &unit,
        &ConsoleApplication {
            id: [14; 16],
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
}

#[test]
fn nested_methods_do_not_capture_the_enclosing_class() {
    for declaration in ["var outerValue = 3;", "const outerValue = 3;"] {
        let source = format!(
            "class Outer {{ {declaration} class Inner {{ function get() {{ return outerValue; }} }} }}"
        );
        assert!(compile_sources(&[&source]).is_err(), "{source}");
    }
}

#[test]
fn saved_construction_rejects_an_unrelated_outer_receiver() {
    let unit = compile_sources(&[r#"
        class Outer { class Inner {} }
        class Other { class Inner {} }
        function make(outer as Outer) { return new outer.Inner(); }
    "#])
    .unwrap();
    let text = unit.to_ir_text();
    let bad = text.replace("Outer.Inner Lang.Object", "Other.Inner Lang.Object");
    assert_ne!(text, bad);
    assert!(parse_unit_ir(&bad).is_err());
}

#[test]
fn outer_subclasses_cannot_replace_the_proven_nested_selector() {
    for member in ["class Inner {}", "var Inner = 1;"] {
        let source =
            format!("class Outer {{ class Inner {{}} }} class Sub extends Outer {{ {member} }}");
        assert!(compile_sources(&[&source]).is_err(), "{source}");
    }
}
