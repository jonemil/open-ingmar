use ciq_compiler::{compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn shadows_preserve_static_contracts_and_complete_saved_application() {
    let unit = checking::compile_sources(&[include_str!(
        "../../../examples/compiler-field-shadowing/app.mc"
    )])
    .unwrap();
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: [10; 16],
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    let class = |name| unit.classes().iter().find(|c| c.name == name).unwrap();
    assert_eq!(class("TypedRoot").fields.len(), 1);
    assert_eq!(class("TypedLeaf").fields.len(), 1);
    assert_eq!(
        class("TypedRoot").fields[0].kind,
        ciq_compiler::FieldKind::Number(11)
    );
    assert_eq!(
        class("TypedLeaf").fields[0].kind,
        ciq_compiler::FieldKind::Initialized {
            value_type: ciq_compiler::ReturnType::String
        }
    );
    assert_eq!(
        class("TypedRoot")
            .methods
            .iter()
            .find(|f| f.name() == "readRoot")
            .unwrap()
            .return_type(),
        ciq_compiler::ReturnType::Integer
    );
}

#[test]
fn public_shadows_and_access_reductions_resolve_across_files() {
    let root = "module M{class Root{var value as Toybox.Lang.Number=1;function read() as Toybox.Lang.Number{return value;}}}";
    for access in ["public", "protected", "private"] {
        let leaf = format!(
            "using M as Models;class Leaf extends Models.Root{{{access} var value as Toybox.Lang.String=\"leaf\";function own() as Toybox.Lang.String{{return value;}}}}function read(x as Models.Root) as Toybox.Lang.Number{{return x.value;}}function make(){{return new Leaf();}}"
        );
        for sources in [vec![root, leaf.as_str()], vec![leaf.as_str(), root]] {
            common::pipeline(checking::compile_sources(&sources).unwrap());
        }
    }
    let source = "class Root{var value=1;}class Middle extends Root{private var value=\"middle\";}class Leaf extends Middle{var value=3;}function make(){return new Leaf();}";
    common::pipeline(compile_sources(&[source]).unwrap());
}

#[test]
fn protected_parent_fields_cannot_be_redeclared() {
    for parent_access in ["protected", "hidden"] {
        for access in ["public", "protected", "private"] {
            let source = format!(
                "class Root{{{parent_access} var value=1;}}class Leaf extends Root{{{access} var value=2;}}"
            );
            let error = compile_sources(&[&source]).unwrap_err().to_string();
            assert!(error.contains("protected inherited field"), "{error}");
        }
    }
    let source = "class Root{var value=1;}class Leaf extends Root{var value=2;}";
    let text = compile_sources(&[source]).unwrap().to_ir_text();
    let bad = text.replace(
        ".class Root Lang.Object\n",
        ".class Root Lang.Object\n.visibility value protected\n",
    );
    assert_ne!(bad, text);
    assert!(
        parse_unit_ir(&bad)
            .unwrap_err()
            .to_string()
            .contains("protected inherited field")
    );
}

#[test]
fn readonly_self_checks_follow_visible_descendant_selectors() {
    for access in ["public", "protected"] {
        for store in ["value=3;", "self.value=3;"] {
            let source = format!(
                "class Root{{var value as Toybox.Lang.Number=1;function write(){{{store}}}}}class Leaf extends Root{{{access} const value as Toybox.Lang.Number=2;}}"
            );
            assert!(
                compile_sources(&[&source])
                    .unwrap_err()
                    .to_string()
                    .contains("constant field")
            );
        }
    }
    let private = "class Root{var value as Toybox.Lang.Number=1;function write(){value=3;}}class Leaf extends Root{private const value as Toybox.Lang.Number=2;}";
    let unit = common::pipeline(checking::compile_sources(&[private]).unwrap());
    let bad = unit
        .to_ir_text()
        .replace(".visibility value private", ".visibility value protected");
    assert!(
        parse_unit_ir(&bad)
            .unwrap_err()
            .to_string()
            .contains("constant field")
    );
    for source in [
        "class Root{var value as Toybox.Lang.Number=1;}class Leaf extends Root{const value as Toybox.Lang.Number=2;}function put(x as Root){x.value=3;}",
        "class Root{var value as Toybox.Lang.Number=1;function write(){var other=self;other.value=3;}}class Leaf extends Root{const value as Toybox.Lang.Number=2;}",
        "class Root{private var value as Toybox.Lang.Number=1;function write(){value=3;}}class Leaf extends Root{const value as Toybox.Lang.Number=2;}",
        "class Root{const value as Toybox.Lang.Number=1;}class Leaf extends Root{var value as Toybox.Lang.Number=2;function write(){value=3;}}",
    ] {
        common::pipeline(checking::compile_sources(&[source]).unwrap());
    }
}

#[test]
fn access_and_structural_witnesses_use_the_static_field_declaration() {
    let source = "class Root{var value as Toybox.Lang.Number=1;}class Leaf extends Root{protected var value as Toybox.Lang.String=\"leaf\";}function f(x as Leaf){return x.value;}";
    assert!(compile_sources(&[source]).is_err());
    let source = "using Toybox.Lang;typedef Record as interface{var value as Lang.Number;};class Root{var value as Lang.Number=1;}class Leaf extends Root{var value as Lang.String=\"leaf\";}function f(x as Record) as Lang.Number{return x.value;}function root(x as Root) as Lang.Number{return f(x);}";
    common::pipeline(checking::compile_sources(&[source]).unwrap());
    let bad = source.replace("function root(x as Root)", "function root(x as Leaf)");
    assert!(compile_sources(&[&bad]).is_err());
}
