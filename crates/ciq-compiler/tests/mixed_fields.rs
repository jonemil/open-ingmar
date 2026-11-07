use ciq_compiler::{compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn receiver_views_private_binding_and_qualification_preserve_complete_application() {
    let unit = checking::compile_sources(&[include_str!(
        "../../../examples/compiler-mixed-fields/app.mc"
    )])
    .unwrap();
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: [13; 16],
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    let text = unit.to_ir_text();
    let bad = text.replace(
        " StaticLeaf slot result=integer",
        " StaticLeaf slot result=string",
    );
    assert!(bad != text);
    assert!(parse_unit_ir(&bad).is_err());
    let asm = unit.lower().to_text();
    assert!(asm.contains("__ciq_private_17_PrivateStaticLeaf_slot"));
}

#[test]
fn instance_and_static_contracts_resolve_across_files() {
    let root = "module M{class Root{var slot as Toybox.Lang.Number=11;function initialize(){}}}";
    let leaf = "using M as Types;class Leaf extends Types.Root{static var slot as Toybox.Lang.String=\"leaf\";function instance() as Toybox.Lang.Number{return slot.abs();}static function stat() as Toybox.Lang.Number{return slot.length();}function qualified() as Toybox.Lang.Number{return Leaf.slot;}}function f(x as Leaf) as Toybox.Lang.Number{return x.slot.abs();}";
    for sources in [vec![root, leaf], vec![leaf, root]] {
        common::pipeline(checking::compile_sources(&sources).unwrap());
    }
    for bad in [
        leaf.replace("return x.slot.abs()", "return x.slot.length()"),
        leaf.replace("return slot.abs()", "return slot.length()"),
        leaf.replace("return slot.length()", "return slot.abs()"),
        leaf.replace("return x.slot.abs()", "return Leaf.slot"),
    ] {
        assert!(compile_sources(&[root, &bad]).is_err(), "{bad}");
    }
}

#[test]
fn instance_child_hides_static_ancestor_in_instance_and_static_scopes() {
    let root = "class Root{static var slot as Toybox.Lang.Number=11;function initialize(){}}";
    let leaf = "class Leaf extends Root{var slot as Toybox.Lang.String=\"leaf\";function read() as Toybox.Lang.Number{return slot.length();}}function f(x as Leaf) as Toybox.Lang.Number{return x.slot.length();}";
    common::pipeline(checking::compile_sources(&[root, leaf]).unwrap());
    for bad in [
        leaf.replace("return slot.length()", "return slot.abs()"),
        leaf.replace("function read()", "static function read()"),
        leaf.replace("return x.slot.length()", "return Leaf.slot.length()"),
    ] {
        assert!(compile_sources(&[root, &bad]).is_err());
    }
}

#[test]
fn readonly_rules_distinguish_scoped_and_explicit_receiver_stores() {
    let base = "class Root{var slot as Toybox.Lang.Number=11;function put(){slot=12;}}class Leaf extends Root{static const slot as Toybox.Lang.String=\"leaf\";function read() as Toybox.Lang.Number{return slot;}}function f(x as Leaf){x.slot=12;}";
    common::pipeline(checking::compile_sources(&[base]).unwrap());
    for body in ["slot=12;", "self.slot=12;", "Leaf.slot=12;"] {
        let bad = base.replace("return slot;", &format!("{body}return slot;"));
        assert!(compile_sources(&[&bad]).is_err(), "{body}");
    }
    let mutable = "class Root{const slot as Toybox.Lang.Number=11;}class Leaf extends Root{static var slot as Toybox.Lang.String=\"leaf\";static function put(){slot=\"changed\";}}";
    common::pipeline(checking::compile_sources(&[mutable]).unwrap());
    assert!(compile_sources(&[&mutable.replace("static function put", "function put")]).is_err());
}

#[test]
fn interface_witnesses_keep_declaration_view_and_private_ancestors_stay_independent() {
    let source = "using Toybox.Lang;typedef S as interface{var slot as Lang.String;};class Root{var slot as Lang.Number=11;}class Leaf extends Root{static var slot as Lang.String=\"leaf\";}function read(x as S) as Lang.String{return x.slot;}function f(x as Leaf) as Lang.String{return read(x);}";
    common::pipeline(checking::compile_sources(&[source]).unwrap());
    assert!(
        compile_sources(&[&source.replace(
            "interface{var slot as Lang.String;}",
            "interface{var slot as Lang.Number;}"
        )])
        .is_err()
    );
    let private = "class Root{private var slot as Toybox.Lang.Number=11;}class Leaf extends Root{static var slot as Toybox.Lang.String=\"leaf\";}function f(){return Leaf.slot;}";
    common::pipeline(checking::compile_sources(&[private]).unwrap());
}

#[test]
fn private_static_binding_keeps_initialization_dependencies() {
    let source = "class Root{var slot as Toybox.Lang.Number=11;}class Leaf extends Root{private static var slot as Toybox.Lang.String=M.read();function read() as Toybox.Lang.Number{return slot;}}module M{var value=f(new Leaf());function read() as Toybox.Lang.String{return \"leaf\";}}function f(x as Leaf) as Toybox.Lang.Number{return x.read();}";
    let error = compile_sources(&[source]).unwrap_err().to_string();
    assert!(error.contains("initializ"), "{error}");
    common::pipeline(compile_sources(&[&source.replace("=M.read();", "=\"leaf\";")]).unwrap());
}

#[test]
fn allocation_backed_class_defaults_are_rejected_until_metadata_matches_sdk() {
    let source = r#"class Root{static var slot as Toybox.Lang.Number=11;}class Leaf extends Root{var slot as Toybox.Lang.String="leaf";function read() as Toybox.Lang.String{return Leaf.slot;}}"#;
    let error = compile_sources(&[source]).unwrap_err().to_string();
    assert!(
        error.contains("allocation-backed instance defaults"),
        "{error}"
    );
    let numeric = "class Root{static var slot as Toybox.Lang.Number=11;}class Leaf extends Root{var slot as Toybox.Lang.Number=22;function read() as Toybox.Lang.Number{return Leaf.slot;}}";
    common::pipeline(checking::compile_sources(&[numeric]).unwrap());
}
