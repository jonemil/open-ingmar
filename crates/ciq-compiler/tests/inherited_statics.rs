use ciq_compiler::{compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn inherited_statics_preserve_complete_application_and_requested_class() {
    // Receiver identity comparisons intentionally use the gradual Object boundary.
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-inherited-statics/app.mc"
    )])
    .unwrap();
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: [11; 16],
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    let text = unit.to_ir_text();
    assert!(text.contains(" Leaf owner"));
    assert!(text.contains(" Leaf value "));
    assert!(
        unit.classes()
            .iter()
            .find(|c| c.name == "Empty")
            .unwrap()
            .fields
            .is_empty()
    );
    for bad in [
        text.replace(" Leaf value result=integer", " Leaf value result=string"),
        text.replace(" Leaf owner", " Leaf initialize"),
    ] {
        assert!(bad != text, "malformed case must change the unit");
        assert!(parse_unit_ir(&bad).is_err());
    }
}

#[test]
fn inherited_static_names_resolve_across_files_and_modules() {
    let root = "module M{class Root{static var value as Toybox.Lang.Number=1;static function take(n as Toybox.Lang.Number) as Toybox.Lang.Number{return n;}static function owner(){return self;}}}";
    let leaf = "using M as Models;class Leaf extends Models.Root{static function read() as Toybox.Lang.Number{return value;}function initialize(){}}function f(){Leaf.take(Leaf.value);return new Leaf();}";
    for sources in [vec![root, leaf], vec![leaf, root]] {
        common::pipeline(checking::compile_sources(&sources).unwrap());
    }
    let dynamic = leaf.replace(
        "function f(){Leaf.take(Leaf.value);return new Leaf();}",
        "function f(x){Leaf.take(x);}",
    );
    assert!(checking::compile_sources(&[root, &dynamic]).is_err());
    // Calling take with no arguments is invalid even when resolved through Leaf.
    assert!(
        compile_sources(&[root, &leaf.replace("Leaf.take(Leaf.value)", "Leaf.take()")]).is_err()
    );
}

#[test]
fn inherited_static_access_readonly_and_instance_method_rules_are_checked() {
    for source in [
        "class Root{private static var value=1;}class Leaf extends Root{}function f(){return Leaf.value;}",
        "class Root{private static function hidden(){}}class Leaf extends Root{}function f(){Leaf.hidden();}",
        "class Root{protected static var value=1;}class Leaf extends Root{}function f(){return Leaf.value;}",
        "class Root{static const value=1;}class Leaf extends Root{}function f(){Leaf.value=2;}",
        "class Root{function f(){}}class Leaf extends Root{}function g(){Leaf.f();}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    for source in [
        "class Root{protected static var value as Toybox.Lang.Number=1;}class Leaf extends Root{static function f() as Toybox.Lang.Number{return Root.value;}}",
        "class Root{static var value=1;}class Leaf extends Root{}function f(){Leaf.value=2;return new Leaf();}",
        "class Root{enum Kind{VALUE=1}}class Leaf extends Root{}function f(){var x=new Leaf();return Leaf.VALUE;}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap());
    }
}

#[test]
fn inherited_field_aliases_cannot_bypass_parent_initialization_readiness() {
    let source = "function make() as Toybox.Lang.Number{return 2;}class Root{static const FIRST as Toybox.Lang.Number=Leaf.LATER;static const LATER as Toybox.Lang.Number=make();}class Leaf extends Root{}function f(){}";
    let error = compile_sources(&[source]).unwrap_err().to_string();
    assert!(
        error.contains("static constant read before required initialization"),
        "{error}"
    );
    let safe = source.replace("=make();", "=2;");
    common::pipeline(compile_sources(&[&safe]).unwrap());
}

#[test]
fn inherited_lookup_and_instance_defaults_include_parent_static_effects() {
    for expression in ["Leaf.owner()", "new Leaf()"] {
        let field = if expression == "new Leaf()" {
            "var instanceValue as Toybox.Lang.Number=VALUE;"
        } else {
            ""
        };
        let source = format!(
            "class Root{{static const VALUE as Toybox.Lang.Number=M.read();{field}static function owner(){{return self;}}}}class Leaf extends Root{{}}module M{{var OBJECT={expression};function read() as Toybox.Lang.Number{{return 1;}}}}function f(){{}}"
        );
        let error = compile_sources(&[&source]).unwrap_err().to_string();
        assert!(error.contains("initializ"), "{error}");
        let safe = source.replace("=M.read();", "=1;");
        common::pipeline(compile_sources(&[&safe]).unwrap());
    }
}
