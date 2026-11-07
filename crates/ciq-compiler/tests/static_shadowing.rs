use ciq_compiler::{compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn static_shadows_preserve_complete_saved_application() {
    let unit = checking::compile_sources(&[include_str!(
        "../../../examples/compiler-static-shadowing/app.mc"
    )])
    .unwrap();
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: [12; 16],
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    let text = unit.to_ir_text();
    let bad = text.replace(
        ".field-static-variable value integer 22",
        ".field-static-constant value integer 22",
    );
    assert!(bad != text, "mutation must change the unit");
    assert!(
        parse_unit_ir(&bad).is_err(),
        "class stores must retain mutable storage"
    );
}

#[test]
fn static_shadows_resolve_own_contracts_across_files_and_aliases() {
    let root = "module M{class Root{static var value as Toybox.Lang.Number=1;static function root() as Toybox.Lang.Number{return value;}}}";
    let leaf = "using M as Models;class Leaf extends Models.Root{static var value as Toybox.Lang.String=\"leaf\";static function leaf() as Toybox.Lang.String{return value;}}function f(){Leaf.value=\"changed\";Models.Root.value=2;}";
    for sources in [vec![root, leaf], vec![leaf, root]] {
        common::pipeline(checking::compile_sources(&sources).unwrap());
    }
    for bad in [
        leaf.replace("Leaf.value=\"changed\"", "Leaf.value=2"),
        leaf.replace("Models.Root.value=2", "Models.Root.value=\"bad\""),
    ] {
        assert!(compile_sources(&[root, &bad]).is_err());
    }
}

#[test]
fn static_visibility_and_readonly_checks_keep_declaring_storage() {
    for access in ["public", "protected", "private"] {
        let source = format!(
            "class Root{{static var value as Toybox.Lang.Number=1;}}class Leaf extends Root{{{access} static var value as Toybox.Lang.String=\"leaf\";static function own() as Toybox.Lang.String{{return value;}}}}"
        );
        common::pipeline(checking::compile_sources(&[&source]).unwrap());
        if access != "public" {
            assert!(
                compile_sources(&[&format!("{source}function f(){{return Leaf.value;}}")]).is_err()
            );
        }
        let protected = source.replace("class Root{static var", "class Root{protected static var");
        assert!(
            compile_sources(&[&protected])
                .unwrap_err()
                .to_string()
                .contains("protected inherited field")
        );
    }
    for source in [
        "class Root{static var value as Toybox.Lang.Number=1;static function put(){value=3;}}class Leaf extends Root{static const value as Toybox.Lang.Number=2;}",
        "class Root{static const value as Toybox.Lang.Number=1;}class Leaf extends Root{static var value as Toybox.Lang.Number=2;static function put(){value=3;}}",
        "class Root{private static var value as Toybox.Lang.Number=1;}class Leaf extends Root{static var value as Toybox.Lang.String=\"leaf\";}",
    ] {
        common::pipeline(checking::compile_sources(&[source]).unwrap());
    }
    let source = "class Root{static var value as Toybox.Lang.Number=1;}class Leaf extends Root{static const value as Toybox.Lang.Number=2;}function f(){Leaf.value=3;}";
    assert!(compile_sources(&[source]).is_err());
}

#[test]
fn readiness_uses_distinct_shadowed_slots() {
    let source = "class Root{static const value as Toybox.Lang.Number=Leaf.value;}class Leaf extends Root{static const value as Toybox.Lang.Number=2;}function f(){return Root.value;}";
    common::pipeline(checking::compile_sources(&[source]).unwrap());
    let cycle = source.replace("=2;", "=Root.value;");
    assert!(compile_sources(&[&cycle]).is_err());
    let bad = compile_sources(&[source]).unwrap().to_ir_text().replace(
        ".class Root Lang.Object\n",
        ".class Root Lang.Object\n.visibility value protected\n",
    );
    assert!(
        parse_unit_ir(&bad)
            .unwrap_err()
            .to_string()
            .contains("protected inherited field")
    );
}

#[test]
fn instance_and_static_bodies_apply_distinct_readonly_shadow_rules() {
    let source = "class Root{static var value as Toybox.Lang.Number=1;function put(){value=3;}}class Leaf extends Root{static const value as Toybox.Lang.Number=2;}";
    assert!(
        compile_sources(&[source])
            .unwrap_err()
            .to_string()
            .contains("constant field")
    );
    let static_body = source.replace("function put()", "static function put()");
    common::pipeline(checking::compile_sources(&[&static_body]).unwrap());
    let bad = compile_sources(&[&static_body])
        .unwrap()
        .to_ir_text()
        .replace(
            ".function put receiver=static",
            ".function put receiver=instance",
        );
    assert!(bad != compile_sources(&[&static_body]).unwrap().to_ir_text());
    assert!(
        parse_unit_ir(&bad)
            .unwrap_err()
            .to_string()
            .contains("constant field")
    );
    let private = source.replace("static const value", "private static const value");
    common::pipeline(checking::compile_sources(&[&private]).unwrap());
}

#[test]
fn class_enum_slots_and_static_variables_retain_separate_declarations() {
    let source = "class Root{enum Kind{ITEM=11}}class Leaf extends Root{static var ITEM as Toybox.Lang.Number=22;}function f(){var original=Root.ITEM;Leaf.ITEM=23;return Leaf.ITEM;}";
    common::pipeline(checking::compile_sources(&[source]).unwrap());
}
