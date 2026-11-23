use ciq_compiler::{compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn ancestor_assignments_preserve_complete_saved_application() {
    let unit = checking::compile_sources(&[include_str!(
        "../../../examples/compiler-ancestor-constructors/app.mc"
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
    for bad in [
        text.replace(
            ".call-class-discard M.Root initialize",
            ".call-class-discard Middle initialize",
        ),
        text.replace(
            ".call-class-discard Middle initialize",
            ".call-class-discard M.Root initialize",
        ),
    ] {
        assert_ne!(bad, text);
        assert!(parse_unit_ir(&bad).is_err());
    }
}

#[test]
fn constructor_summaries_use_declaring_slots_and_ignore_file_order() {
    let root = "module M{class Root{private var n as Toybox.Lang.Number;function initialize(){n=1;}function read() as Toybox.Lang.Number{return n;}}}";
    let leaf = "using M as T;class Leaf extends T.Root{private var n as Toybox.Lang.String;function initialize(){T.Root.initialize();n=\"leaf\";}}function make(){return new Leaf();}";
    for sources in [vec![root, leaf], vec![leaf, root]] {
        common::pipeline(checking::compile_sources(&sources).unwrap());
    }
    for bad in [
        leaf.replace("T.Root.initialize();", ""),
        leaf.replace("n=\"leaf\";", ""),
    ] {
        assert!(compile_sources(&[root, &bad]).is_err(), "{bad}");
    }
    // An unused empty child declaration is not an allocation proof.
    common::pipeline(compile_sources(&[root, "class Leaf extends M.Root{}"]).unwrap());
    assert!(
        compile_sources(&[
            root,
            "class Leaf extends M.Root{}function make(){return new Leaf();}"
        ])
        .is_err()
    );
}

#[test]
fn reads_exposure_recursion_and_virtual_parent_calls_require_ready_fields() {
    let root = "class Root{var n as Toybox.Lang.Number;function initialize(){n=1;}function read() as Toybox.Lang.Number{return n;}}";
    for prefix in [
        "Toybox.System.println(n);",
        "var alias=self;",
        "read();",
        "Root.read();",
        "Leaf.initialize();",
    ] {
        let leaf = format!(
            "class Leaf extends Root{{function initialize(){{{prefix}Root.initialize();}}}}function make(){{return new Leaf();}}"
        );
        assert!(compile_sources(&[root, &leaf]).is_err(), "{leaf}");
    }
    let virtual_call = "class Root{var n as Toybox.Lang.Number;function initialize(){n=1;read();}function read() as Toybox.Lang.Number{return n;}}class Leaf extends Root{var child as Toybox.Lang.Number;function initialize(){Root.initialize();child=2;}function read() as Toybox.Lang.Number{return child;}}function make(){return new Leaf();}";
    assert!(compile_sources(&[virtual_call]).is_err());
    common::pipeline(
        compile_sources(&[
            &virtual_call.replace("Root.initialize();child=2;", "child=2;Root.initialize();")
        ])
        .unwrap(),
    );
}

#[test]
fn parent_initializer_reads_use_the_callers_existing_readiness() {
    let source = "class Root{var n as Toybox.Lang.Number;function initialize(){n=1;}}class Middle extends Root{var m as Toybox.Lang.Number;function initialize(){m=n+1;}}class Leaf extends Middle{function initialize(){Root.initialize();Middle.initialize();}}function make(){return new Leaf();}";
    common::pipeline(compile_sources(&[source]).unwrap());
    for bad in [
        source.replace("return new Leaf();", "return new Middle();"),
        source.replace(
            "Root.initialize();Middle.initialize();",
            "Middle.initialize();Root.initialize();",
        ),
    ] {
        assert!(compile_sources(&[&bad]).is_err(), "{bad}");
    }
}

#[test]
fn allocation_defaults_and_argument_effects_cannot_read_unready_ancestors() {
    let root = "class Root{var n as Toybox.Lang.Number;function initialize(v as Toybox.Lang.Number){n=v;}}";
    for leaf in [
        "class Leaf extends Root{var copy as Toybox.Lang.Number=n;function initialize(){Root.initialize(1);}}",
        "class Leaf extends Root{function initialize(){Root.initialize(n);}}",
        "class Leaf extends Root{var copy=self;function initialize(){Root.initialize(1);}}",
        "class Leaf extends Root{function initialize(){Root.initialize(true?1:2);}}",
    ] {
        assert!(
            compile_sources(&[root, leaf, "function make(){return new Leaf();}"]).is_err(),
            "{leaf}"
        );
    }
}

#[test]
fn parent_initialization_must_finish_before_shutdown() {
    let source = "class Root{var n as Toybox.Lang.Number;function initialize(){n=1;Toybox.System.exit();}}class Leaf extends Root{var child as Toybox.Lang.Number;function initialize(){Root.initialize();child=2;}}function make(){return new Leaf();}";
    assert!(
        compile_sources(&[source])
            .unwrap_err()
            .to_string()
            .contains("shutdown")
    );
    common::pipeline(
        compile_sources(&[
            &source.replace("Root.initialize();child=2;", "child=2;Root.initialize();")
        ])
        .unwrap(),
    );
}
