use ciq_compiler::{compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn qualified_instance_application_preserves_saved_stages_and_branch_capture() {
    let unit = checking::compile_sources(&[include_str!(
        "../../../examples/compiler-qualified-instance-calls/app.mc"
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
    assert!(text.contains(".begin-call"));
    assert!(text.contains("M.Root pick"));
    assert!(
        unit.lower()
            .to_text()
            .contains("@__ciq_member___ciq_private_4_Leaf_secret")
    );
}

#[test]
fn qualified_methods_keep_access_signature_and_receiver_checks_across_files() {
    let root = "module M{class Root{protected function pick(n as Toybox.Lang.Number) as Toybox.Lang.Number{return n;}function initialize(n as Toybox.Lang.Number){}}}";
    let leaf = "using M as T;class Leaf extends T.Root{function initialize(){T.Root.initialize(1);}function read() as Toybox.Lang.Number{return T.Root.pick(2);}function own() as Toybox.Lang.Number{return Leaf.pick(3);}}function make(){return new Leaf();}";
    for sources in [vec![root, leaf], vec![leaf, root]] {
        common::pipeline(checking::compile_sources(&sources).unwrap());
    }
    for bad in [
        leaf.replace("return T.Root.pick(2)", "return T.Root.pick(\"x\")"),
        leaf.replace("T.Root.initialize(1)", "T.Root.initialize()"),
        leaf.replace("function read()", "static function read()"),
        format!("{leaf}function outside(){{T.Root.pick(1);}}"),
        format!("{leaf}class Other{{function read(){{T.Root.pick(1);}}}}"),
    ] {
        assert!(compile_sources(&[root, &bad]).is_err(), "{bad}");
    }
    assert!(
        compile_sources(&[
            &root.replace("protected function pick", "private function pick"),
            leaf
        ])
        .is_err()
    );
    let unit = compile_sources(&[root, leaf]).unwrap();
    let text = unit.to_ir_text();
    let bad = text.replace(
        ".function read receiver=instance",
        ".function read receiver=static",
    );
    assert_ne!(bad, text);
    assert!(parse_unit_ir(&bad).is_err());
}

#[test]
fn ordinary_parent_initializers_are_not_automatic_allocation_hooks() {
    for parent in [
        "function initialize(){Toybox.System.println(10);}",
        "function initialize(n as Toybox.Lang.Number){}",
        "private function initialize(){}",
    ] {
        let source = format!(
            "class Root{{{parent}}}class Leaf extends Root{{}}function make(){{return new Leaf();}}"
        );
        common::pipeline(compile_sources(&[&source]).unwrap());
    }
    let private = "class Root{private function initialize(){}}class Leaf extends Root{function initialize(){Root.initialize();}}";
    assert!(compile_sources(&[private]).is_err());
}

#[test]
fn static_own_initializer_calls_keep_the_static_receiver_without_allocation() {
    let source = "class C{var field as Toybox.Lang.Number=10;function initialize(){Toybox.System.println(field);}static function run(){C.initialize();}}";
    let unit = common::pipeline(compile_sources(&[source]).unwrap());
    let assembly = unit.lower().to_text();
    assert!(!assembly.contains("newc"));
    assert!(assembly.contains("@__ciq_member_initialize"));
    for bad in [
        source
            .replace("C.initialize();", "C.pick();")
            .replace("function initialize()", "function pick()"),
        format!("{source}function outside(){{C.initialize();}}"),
        format!("{source}class D extends C{{static function f(){{C.initialize();}}}}"),
    ] {
        assert!(compile_sources(&[&bad]).is_err(), "{bad}");
    }
}

#[test]
fn qualified_calls_preserve_pending_field_and_ancestor_escape_checks() {
    let source = "class Holder{var n as Toybox.Lang.Number;function initialize(){Holder.read();n=1;}function read() as Toybox.Lang.Number{return n;}}";
    assert!(
        compile_sources(&[source])
            .unwrap_err()
            .to_string()
            .contains("required fields")
    );
    common::pipeline(
        compile_sources(&[&source.replace("Holder.read();n=1", "n=1;Holder.read()")]).unwrap(),
    );
    let captured = source
        .replace("Holder.read()", "Holder.pick(true?1:2)")
        .replace("function read()", "function pick(x as Toybox.Lang.Number)");
    assert!(
        compile_sources(&[&captured])
            .unwrap_err()
            .to_string()
            .contains("required fields")
    );
    let ancestor = "class Root{var first as Toybox.Lang.Number=1+1;var last as Toybox.Lang.Number=Root.read();function read() as Toybox.Lang.Number{return first;}}class Leaf extends Root{}function make(){return new Leaf();}";
    assert!(compile_sources(&[ancestor]).is_err());
    // A static call does not expose the instance, even before its fields are ready.
    let safe = source.replace(
        "function read() as Toybox.Lang.Number{return n;}",
        "static function read() as Toybox.Lang.Number{return 2;}",
    );
    common::pipeline(compile_sources(&[&safe]).unwrap());
}

#[test]
fn qualified_effects_include_parent_bodies_and_their_virtual_calls() {
    let source = "class Root{function initialize(){Toybox.System.exit();}function pick() as Toybox.Lang.Number{return 1;}function forward() as Toybox.Lang.Number{return pick();}}class Leaf extends Root{function pick() as Toybox.Lang.Number{Toybox.System.exit();return 2;}function run(){Root.forward();}}class Holder{var target as Root;function initialize(x as Leaf){x.run();self.target=x;}}";
    assert!(
        compile_sources(&[source])
            .unwrap_err()
            .to_string()
            .contains("shutdown")
    );
    let safe = source.replace("x.run();self.target=x", "self.target=x;x.run()");
    common::pipeline(compile_sources(&[&safe]).unwrap());
    let parent = source.replace("Root.forward();", "Root.initialize();");
    assert!(
        compile_sources(&[&parent])
            .unwrap_err()
            .to_string()
            .contains("shutdown")
    );
    // Merely allocating a subclass must not add the unused parent's initialize body.
    let allocation = "class Root{function initialize(){Toybox.System.exit();}}class Leaf extends Root{}class Holder{var target as Root;function initialize(){var x=new Leaf();self.target=x;}}";
    common::pipeline(compile_sources(&[allocation]).unwrap());
}
