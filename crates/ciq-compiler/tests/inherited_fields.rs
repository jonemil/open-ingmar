use ciq_compiler::{compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn inherited_fields_preserve_complete_saved_application() {
    let unit = checking::compile_sources(&[include_str!(
        "../../../examples/compiler-inherited-fields/app.mc"
    )])
    .unwrap();
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: [9; 16],
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    let text = unit.to_ir_text();
    let (_, leaf) = text.split_once(".class Leaf ").unwrap();
    assert!(
        !leaf
            .split(".end-class")
            .next()
            .unwrap()
            .contains(".field-initialized root")
    );
    for bad in [
        text.replace(
            ".field-initialized root integer",
            ".field-initialized root string",
        ),
        text.replace(".visibility step protected", ".visibility step private"),
        text.replace(".field-initialized root integer\n", ""),
    ] {
        assert_ne!(bad, text);
        assert!(parse_unit_ir(&bad).is_err());
    }
}

#[test]
fn inherited_contract_inference_is_independent_of_file_order() {
    let root = "class Root{protected var value as Toybox.Lang.Number=1+2;var text as Toybox.Lang.String=\"root\";}";
    let leaf = "class Leaf extends Root{var next as Toybox.Lang.Number=value+1;function valueOf() as Toybox.Lang.Number{return value+next;}function change(){text=\"leaf\";}}function make(){return new Leaf();}";
    for sources in [vec![root, leaf], vec![leaf, root]] {
        common::pipeline(checking::compile_sources(&sources).unwrap());
    }
    common::pipeline(
        compile_sources(&["class A{var n=1;}class B extends A{}function f(){return new B();}"])
            .unwrap(),
    );
}

#[test]
fn inherited_fields_preserve_access_readonly_and_invariant_contracts() {
    let declarations = "class Root{protected var hidden=1;private var secret=2;var value as Toybox.Lang.Number=3;const label=\"root\";}class Leaf extends Root{function read() as Toybox.Lang.Number{return hidden;}}";
    for body in [
        "return x.hidden;",
        "return x.secret;",
        "x.label=\"bad\";",
        "x.value=\"bad\";",
    ] {
        let source = format!("{declarations}function f(x as Leaf){{{body}}}");
        assert!(compile_sources(&[&source]).is_err(), "{source}");
    }
    let bad = declarations.replace("return hidden;", "label=\"bad\";return hidden;");
    assert!(compile_sources(&[&bad]).is_err());
    let bad = declarations.replace("return hidden;", "return secret;");
    assert!(compile_sources(&[&bad]).is_err());
    let shadow = "class Root{var value=1;}class Leaf extends Root{var value=2;}";
    common::pipeline(compile_sources(&[shadow]).unwrap());
    let private = "class Root{private var value as Toybox.Lang.Number=1;function read() as Toybox.Lang.Number{return value;}}class Leaf extends Root{private var value as Toybox.Lang.String=\"leaf\";function readLeaf() as Toybox.Lang.String{return value;}}function make(){return new Leaf();}";
    common::pipeline(checking::compile_sources(&[private]).unwrap());
}

#[test]
fn inherited_storage_can_supply_public_structural_fields() {
    let source = "using Toybox.Lang;typedef Record as interface{var value as Lang.Number;};class Root{var value as Lang.Number=1;}class Leaf extends Root{}function f(x as Record) as Lang.Number{return x.value;}function g(x as Leaf) as Lang.Number{return f(x);}";
    common::pipeline(checking::compile_sources(&[source]).unwrap());
}

#[test]
fn unsupported_parent_initialization_and_receiver_escape_stay_explicit() {
    for parent in [
        "var value as Toybox.Lang.Number;function initialize(){value=1;}",
        "var value=1;function initialize(){Toybox.System.println(10);}",
        "var value=1;function initialize(n as Toybox.Lang.Number){}",
        "var value=self;",
        "var value=f();function f() as Toybox.Lang.Number{return 1;}",
    ] {
        let source = format!(
            "class Root{{{parent}}}class Leaf extends Root{{}}function f(){{return new Leaf();}}"
        );
        assert!(compile_sources(&[&source]).is_err(), "{source}");
    }
}

#[test]
fn inherited_allocation_effects_participate_in_shared_initialization_safety() {
    let source = "class Root{var value as Toybox.Lang.Number=Factory.read();}class Leaf extends Root{}module Factory{var BOX=new Leaf();var READY as Toybox.Lang.Number=1+1;function read() as Toybox.Lang.Number{return READY;}}function f(){}";
    let error = compile_sources(&[source]).unwrap_err().to_string();
    assert!(error.contains("initializ"), "{error}");
    let safe = source.replace("=Factory.read();", "=1;");
    common::pipeline(compile_sources(&[&safe]).unwrap());
}
