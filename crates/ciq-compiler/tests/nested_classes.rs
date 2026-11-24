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
    let bad = text.replace(
        "source(Outer.Inner@Lang.Object)",
        "source(Other.Inner@Lang.Object)",
    );
    assert_ne!(text, bad);
    assert!(parse_unit_ir(&bad).is_err());
}

#[test]
fn saved_construction_cannot_drop_a_runtime_child() {
    let unit = compile_sources(&[r#"
        class Outer { class Inner {} }
        class Sub extends Outer { class Inner {} }
        function make(x as Outer) { return new x.Inner(); }
    "#])
    .unwrap();
    let text = unit.to_ir_text();
    let dropped = text
        .lines()
        .map(|line| {
            if line.starts_with(".begin-source ") || line.starts_with(".new-source ") {
                line.replace(
                    "source-union(source(Outer.Inner@Lang.Object)|source(Sub.Inner@Lang.Object))",
                    "source(Outer.Inner@Lang.Object)",
                )
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let error = parse_unit_ir(&dropped).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("declared nested construction alternatives"),
        "{error}"
    );
}

#[test]
fn every_runtime_child_must_accept_the_initializer_arguments() {
    let source = r#"
        class Outer { class Inner { function initialize(n as Toybox.Lang.Number) {} } }
        class Sub extends Outer { class Inner { function initialize(n as Toybox.Lang.String) {} } }
        function make(x as Outer) { return new x.Inner(1); }
    "#;
    let error = compile_sources(&[source]).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Sub.Inner.initialize argument type"),
        "{error}"
    );
}

#[test]
fn polymorphic_construction_requires_a_shared_initializer_selector() {
    let source = r#"
        class Outer { class Inner {} }
        class Sub extends Outer { class Inner { private function initialize() {} } }
        function make(x as Outer) { return new x.Inner(); }
    "#;
    let error = compile_sources(&[source]).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("cannot dispatch to private initializer Sub.Inner.initialize"),
        "{error}"
    );
}

#[test]
fn nested_selection_tracks_declared_receiver_unions_and_requires_the_full_path() {
    let source = r#"
        class Left { class Middle { class Leaf {} } }
        class Right { class Middle { class Leaf {} } }
        function make(outer as Left or Right) as Left.Middle.Leaf or Right.Middle.Leaf {
            return new outer.Middle.Leaf();
        }
    "#;
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    let missing = source
        .replace(
            "class Right { class Middle { class Leaf {} } }",
            "class Right { class Middle {} }",
        )
        .replace(" or Right.Middle.Leaf", "");
    let error = compile_sources(&[&missing]).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("full selector path on every alternative"),
        "{error}"
    );
}

#[test]
fn nested_value_collisions_are_rejected() {
    assert!(
        compile_sources(&[
            "class Outer { class Inner {} } class Sub extends Outer { var Inner = 1; }"
        ])
        .is_err()
    );
}
