use ciq_compiler::{ParameterType, ReturnType, compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn mixed_methods_preserve_complete_application_and_saved_receiver_contracts() {
    let unit = checking::compile_sources(&[include_str!(
        "../../../examples/compiler-static-method-shadowing/app.mc"
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
    assert!(text.contains(".function pick parameters=1 receiver=static parameter-types=string"));
    assert!(
        parse_unit_ir(&text.replace(".visibility pick private", ".visibility pick public"))
            .is_err()
    );
    let private = unit
        .classes()
        .iter()
        .find(|c| c.name == "PrivateLeaf")
        .unwrap();
    let own = private.methods.iter().find(|m| m.name() == "own").unwrap();
    assert_eq!(own.return_type(), ReturnType::Integer);
    assert!(
        unit.lower()
            .to_text()
            .contains("@__ciq_member___ciq_private_11_PrivateLeaf_pick")
    );
}

#[test]
fn instance_precedence_and_static_body_lookup_work_across_files() {
    let root = "module M{class Root{function pick(n as Toybox.Lang.Number) as Toybox.Lang.Number{return n;}}}";
    let leaf = "using M as T;class Leaf extends T.Root{private static function pick(s as Toybox.Lang.String) as Toybox.Lang.String{return s;}function read() as Toybox.Lang.Number{return pick(1);}function qualified() as Toybox.Lang.Number{return Leaf.pick(1);}static function stat() as Toybox.Lang.String{return pick(\"x\");}static function through(x as Leaf) as Toybox.Lang.Number{return x.pick(1);}}function outside(x as Leaf) as Toybox.Lang.Number{return x.pick(1);}";
    for sources in [vec![root, leaf], vec![leaf, root]] {
        common::pipeline(checking::compile_sources(&sources).unwrap());
    }
    for bad in [
        leaf.replace("return pick(1)", "return pick(\"x\")"),
        leaf.replace("return pick(\"x\")", "return pick(1)"),
        leaf.replace("return Leaf.pick(1)", "return Leaf.pick(\"x\")"),
        leaf.replace("return x.pick(1)", "return Leaf.pick(1)"),
        leaf.replace("private static function", "public static function"),
    ] {
        assert!(checking::compile_sources(&[root, &bad]).is_err(), "{bad}");
    }
}

#[test]
fn static_parents_start_independent_families_and_stop_signature_inheritance() {
    let source = "using Toybox.Lang;class Root{static function pick(n as Lang.Number) as Lang.Number{return n;}}class StaticChild extends Root{static function pick(s as Lang.String) as Lang.String{return s;}}class InstanceChild extends Root{function pick(s as Lang.String) as Lang.String{return s;}}class DynamicChild extends Root{function pick(x){return x;}}";
    let unit = common::pipeline(compile_sources(&[source]).unwrap());
    let dynamic = &unit.classes()[3].methods[0];
    assert_eq!(dynamic.parameter_types(), &[ParameterType::Any]);
    assert_eq!(dynamic.return_type(), ReturnType::Any);
    let mixed = "using Toybox.Lang;class Root{function pick(n as Lang.Number) as Lang.Number{return n;}}class Middle extends Root{static function pick(n){return n;}}class Leaf extends Middle{function pick(s as Lang.String) as Lang.String{return s;}}";
    let unit = common::pipeline(checking::compile_sources(&[mixed]).unwrap());
    assert_eq!(
        unit.classes()[1].methods[0].parameter_types(),
        &[ParameterType::Integer]
    );
    let bad = mixed.replace(
        "static function pick(n)",
        "static function pick(n as Lang.String) as Lang.String",
    );
    assert!(compile_sources(&[&bad]).is_err());
    let blocked = "class Root{static function pick() as Toybox.Lang.Number{return 1;}}class Leaf extends Root{function pick() as Toybox.Lang.String{return \"x\";}static function read(){return pick();}}";
    assert!(compile_sources(&[blocked]).is_err());
}

#[test]
fn effects_use_private_static_binding_and_keep_instance_slots_cold() {
    let source = "class Root{function pick() as Toybox.Lang.Number{return 1;}}class Leaf extends Root{private static function pick() as Toybox.Lang.String{Toybox.System.exit();return \"x\";}function forward() as Toybox.Lang.Number{return pick();}}class Holder{var target as Root;function initialize(x as Leaf){x.pick();self.target=x;}}";
    common::pipeline(compile_sources(&[source]).unwrap());
    let bad = source.replace("x.pick();self.target=x", "x.forward();self.target=x");
    assert!(
        compile_sources(&[&bad])
            .unwrap_err()
            .to_string()
            .contains("shutdown")
    );
    let unrelated = "module M{var INPUT as Leaf or Null=null;var FIRST=f(INPUT);var LATER as Toybox.Lang.Number=1+1;function later() as Toybox.Lang.Number{return LATER;}}class Root{function pick() as Toybox.Lang.Number{return 1;}}class Leaf extends Root{static var seed as Toybox.Lang.Number=M.later();static function pick() as Toybox.Lang.Number{return 2;}}function f(x as Leaf or Null) as Toybox.Lang.Number{if(x==null){return 0;}return x.pick();}";
    common::pipeline(compile_sources(&[unrelated]).unwrap());
    let private = unrelated.replace("static function pick() as Toybox.Lang.Number{return 2;}", "private static function pick() as Toybox.Lang.Number{return 2;}function forward() as Toybox.Lang.Number{return pick();}")
        .replace("return x.pick();", "return x.forward();");
    assert!(
        compile_sources(&[&private])
            .unwrap_err()
            .to_string()
            .contains("initializ")
    );
}

#[test]
fn qualified_calls_follow_runtime_static_body_for_shutdown_effects() {
    let source = "class Root{function pick() as Toybox.Lang.Number{return 1;}}class Leaf extends Root{static function pick() as Toybox.Lang.Number{Toybox.System.exit();return 2;}function forward() as Toybox.Lang.Number{return Leaf.pick();}}class Holder{var target as Root;function initialize(x as Leaf){x.forward();self.target=x;}}";
    assert!(
        compile_sources(&[source])
            .unwrap_err()
            .to_string()
            .contains("shutdown")
    );
    let safe = source.replace("x.forward();self.target=x", "self.target=x;x.forward()");
    common::pipeline(compile_sources(&[&safe]).unwrap());
    let root_exit = source
        .replace("return 1;", "Toybox.System.exit();return 1;")
        .replace("Toybox.System.exit();return 2", "return 2");
    common::pipeline(compile_sources(&[&root_exit]).unwrap());
}

#[test]
fn allocation_summaries_include_source_parent_static_initialization() {
    let source = "module M{var FIRST=new Leaf();var LATER as Toybox.Lang.Number=1+1;function later() as Toybox.Lang.Number{return LATER;}}class Root{static var seed as Toybox.Lang.Number=M.later();static function pick() as Toybox.Lang.Number{return seed;}}class Leaf extends Root{}";
    assert!(
        compile_sources(&[source])
            .unwrap_err()
            .to_string()
            .contains("initializ")
    );
    common::pipeline(compile_sources(&[&source.replace("=M.later();", "=1;")]).unwrap());
}
