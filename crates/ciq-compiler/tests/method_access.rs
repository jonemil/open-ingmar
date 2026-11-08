use ciq_compiler::{compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn scoped_lookup_preserves_complete_application_and_saved_contracts() {
    let unit = checking::compile_sources(&[include_str!(
        "../../../examples/compiler-method-access/app.mc"
    )])
    .unwrap();
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: [10; 16],
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    let text = unit.to_ir_text();
    assert!(
        parse_unit_ir(&text.replace(".visibility pick private", ".visibility pick protected"))
            .is_err()
    );
    let assembly = unit.lower().to_text();
    assert!(assembly.contains("@__ciq_member___ciq_private_11_PrivateLeaf_pick"));
    // Private self calls bind a selector; explicit receiver calls retain the
    // ordinary selector even when checking uses a private String signature.
    let through = unit
        .classes()
        .iter()
        .find(|c| c.name == "PrivateLeaf")
        .unwrap()
        .methods
        .iter()
        .find(|m| m.name() == "through")
        .unwrap();
    assert!(through.operations().iter().any(|o| matches!(&o.operation,
        ciq_compiler::Operation::ObjectCall { member, result_type: ciq_compiler::ReturnType::String, .. }
            if member == "pick")));
}

#[test]
fn public_parent_and_private_child_contracts_resolve_across_files_and_aliases() {
    let root = "module M{class Root{function pick(n as Toybox.Lang.Number) as Toybox.Lang.Number{return n;}}}";
    let leaf = "using M as Types;class Leaf extends Types.Root{private function pick(s as Toybox.Lang.String) as Toybox.Lang.String{return s;}function own() as Toybox.Lang.String{return pick(\"x\");}function through(x as Leaf) as Toybox.Lang.String{return x.pick(\"x\");}}function outside(x as Leaf) as Toybox.Lang.Number{return x.pick(1);}";
    for sources in [vec![root, leaf], vec![leaf, root]] {
        common::pipeline(checking::compile_sources(&sources).unwrap());
    }
    for bad in [
        leaf.replace("return x.pick(1)", "return x.pick(\"x\")"),
        leaf.replace("return pick(\"x\")", "return pick(1)"),
        leaf.replace("private function pick", "protected function pick"),
    ] {
        assert!(checking::compile_sources(&[root, &bad]).is_err(), "{bad}");
    }
    let private_only = "class C{private function pick() as Toybox.Lang.Number{return 1;}}function outside(x as C){return x.pick();}";
    assert!(compile_sources(&[private_only]).is_err());
}

#[test]
fn nonprivate_overrides_inherit_annotations_across_private_intermediate_methods() {
    let source = "class Root{function pick(n as Toybox.Lang.Number) as Toybox.Lang.Number{return n;}}class Middle extends Root{private function pick(s as Toybox.Lang.String) as Toybox.Lang.String{return s;}}class Leaf extends Middle{protected function pick(n){return n;}}function outside(x as Leaf) as Toybox.Lang.Number{return x.pick(1);}";
    let unit = common::pipeline(checking::compile_sources(&[source]).unwrap());
    let method = &unit.classes()[2].methods[0];
    assert_eq!(
        method.parameter_types(),
        &[ciq_compiler::ParameterType::Integer]
    );
    assert_eq!(method.return_type(), ciq_compiler::ReturnType::Integer);
    assert!(
        compile_sources(&[&source.replace(
            "protected function pick(n)",
            "protected function pick(n as Toybox.Lang.String)"
        )])
        .is_err()
    );
    common::pipeline(compile_sources(&["class Root{function pick(n){return n;}}class Leaf extends Root{private function pick(){return 2;}}"] ).unwrap());
}

#[test]
fn interface_witnesses_use_declarations_without_imposing_public_visibility() {
    let source = "using Toybox.Lang;typedef Text as interface{function pick(s as Lang.String) as Lang.String;};class Root{function pick(n as Lang.Number) as Lang.Number{return n;}}class Leaf extends Root{private function pick(s as Lang.String) as Lang.String{return s;}}function witness(x as Leaf) as Text{return x;}";
    common::pipeline(checking::compile_sources(&[source]).unwrap());
    let wrong = source.replace(
        "interface{function pick(s as Lang.String) as Lang.String;}",
        "interface{function pick(s as Lang.Number) as Lang.Number;}",
    );
    assert!(checking::compile_sources(&[&wrong]).is_err());
    let comparator = "using Toybox.Lang;class C{private function compare(a as Lang.Object,b as Lang.Object) as Lang.Number{return 0;}}function witness(x as C) as Lang.Comparator{return x;}";
    common::pipeline(checking::compile_sources(&[comparator]).unwrap());
}

#[test]
fn effects_follow_runtime_selectors_instead_of_private_checked_contracts() {
    let source = "class Root{function pick() as Toybox.Lang.Number{return 1;}function forward() as Toybox.Lang.Number{return pick();}}class Leaf extends Root{private function pick() as Toybox.Lang.String{Toybox.System.exit();return \"private\";}function through(x as Leaf) as Toybox.Lang.String{return x.pick();}}class Holder{var target as Root;function initialize(x as Leaf){x.pick();x.through(x);self.target=x;}}";
    common::pipeline(compile_sources(&[source]).unwrap());
    let dangerous = source.replace(
        "function through(x as Leaf) as Toybox.Lang.String{return x.pick();}",
        "function through(x as Leaf) as Toybox.Lang.String{return pick();}",
    );
    assert!(
        compile_sources(&[&dangerous])
            .unwrap_err()
            .to_string()
            .contains("shutdown")
    );
    let protected = source.replace("private function pick() as Toybox.Lang.String{Toybox.System.exit();return \"private\";}", "protected function pick() as Toybox.Lang.Number{Toybox.System.exit();return 2;}")
        .replace("function through(x as Leaf) as Toybox.Lang.String", "function through(x as Leaf) as Toybox.Lang.Number");
    common::pipeline(
        compile_sources(&[&protected.replace("x.pick();x.through(x);", "x.pick();")]).unwrap(),
    );
    assert!(
        compile_sources(&[&protected.replace("x.pick();x.through(x);", "x.forward();")])
            .unwrap_err()
            .to_string()
            .contains("shutdown")
    );
}
