use ciq_compiler::{ClassBase, SourceClass, compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn superclass_identities_preserve_complete_saved_pipelines() {
    let unit = checking::compile_sources(&[include_str!(
        "../../../examples/compiler-inheritance-identities/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".class Models.Leaf Lang.Object parent=Models.Middle"));
    assert!(text.contains("Models.Leaf>Models.Middle>Models.Root@Lang.Object"));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: [7; 16],
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    let leaf = SourceClass {
        name: "Leaf".into(),
        ancestors: vec!["Middle".into(), "Root".into()],
        base: ClassBase::LangObject,
    };
    let root = SourceClass {
        name: "Root".into(),
        ancestors: Vec::new(),
        base: ClassBase::LangObject,
    };
    assert!(root.accepts(&leaf));
    assert!(!leaf.accepts(&root));
}

#[test]
fn superclass_names_resolve_lexically_through_aliases_roots_and_files() {
    for sources in [
        vec!["class Leaf extends Root{}", "class Root{}"],
        vec!["using Toybox.WatchUi;module M{class View{}class Leaf extends View{}}"],
        vec![
            "using M as Types;class Leaf extends Types.Root{}",
            "module M{class Root{}}",
        ],
        vec!["class Root{}module M{class Leaf extends $.Root{}class Root{}}"],
        vec!["module M{class Leaf extends Root{}class Root{}}"],
    ] {
        common::pipeline(compile_sources(&sources).unwrap_or_else(|e| panic!("{sources:?}: {e}")));
    }
}

#[test]
fn invalid_graphs_and_unimplemented_initialization_are_rejected() {
    for source in [
        "class A extends Missing{}",
        "class A extends A{}",
        "class A extends B{}class B extends A{}",
        "class A extends B{}class B extends C{}class C extends A{}",
        "module M{}class A extends M{}",
        "class A{function initialize(){Toybox.System.println(1);}}class B extends A{}function f(){return new B();}",
        "class A{function initialize(n as Toybox.Lang.Number){}}class B extends A{}function f(){return new B();}",
        "class A{private function initialize(){}}class B extends A{}function f(){return new B();}",
        "class A{}class B extends A{}function f(a as A) as B{return a;}",
        "class A{function f(){return 1;}}class B extends A{function f(n){return n;}}",
        "using Toybox.Lang;class A{}class B extends A{}function take(a as Lang.Array<A>){}function f(b as Lang.Array<B>){take(b);}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let chain = |count| {
        (0..count)
            .map(|n| {
                if n == 0 {
                    "class C0{}".into()
                } else {
                    format!("class C{n} extends C{}{{}}", n - 1)
                }
            })
            .collect::<String>()
    };
    common::pipeline(compile_sources(&[&chain(64)]).unwrap());
    assert!(compile_sources(&[&chain(65)]).is_err());
}

#[test]
fn saved_ancestry_must_match_the_declaration_graph() {
    let text = compile_sources(&[include_str!(
        "../../../examples/compiler-inheritance-identities/app.mc"
    )])
    .unwrap()
    .to_ir_text();
    for bad in [
        text.replace("parent=Models.Middle", "parent=Missing"),
        text.replace("parent=Models.Middle", "parent=Models.Leaf"),
        text.replace(
            "Models.Leaf>Models.Middle>Models.Root",
            "Models.Leaf>Models.Root",
        ),
        text.replace(
            "Models.Leaf>Models.Middle>Models.Root",
            "Models.Leaf>Models.Leaf",
        ),
        text.replace(
            ".class Models.Root Lang.Object",
            ".class Models.Root WatchUi.View",
        ),
    ] {
        assert_ne!(bad, text);
        assert!(parse_unit_ir(&bad).is_err(), "{bad}");
    }
}
