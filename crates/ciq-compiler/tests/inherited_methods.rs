use ciq_compiler::{compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn inherited_methods_preserve_complete_application_and_saved_contracts() {
    let unit = checking::compile_sources(&[include_str!(
        "../../../examples/compiler-inherited-methods/app.mc"
    )])
    .unwrap();
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: [8; 16],
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    let text = unit.to_ir_text();
    let method = unit
        .classes()
        .iter()
        .find(|c| c.name == "Leaf")
        .unwrap()
        .methods
        .iter()
        .find(|f| f.name() == "value")
        .unwrap();
    assert_eq!(
        method.parameter_types(),
        &[ciq_compiler::ParameterType::Integer]
    );
    assert_eq!(method.return_type(), ciq_compiler::ReturnType::Integer);
    let bad = text.replacen(
        ".function value parameters=1 receiver=instance",
        ".function value parameters=1 parameter-types=string receiver=instance",
        1,
    );
    assert_ne!(bad, text);
    assert!(parse_unit_ir(&bad).is_err());
}

#[test]
fn lookup_and_partial_override_contracts_work_across_files() {
    let root = "using Toybox.Lang;module M{class Root{function value(n as Lang.Number) as Lang.Number{return n+1;}protected function step(n as Lang.Number) as Lang.Number{return n;}function initialize(){}}}";
    let leaf = "using Toybox.Lang;using M as Types;class Leaf extends Types.Root{function value(n){return n+2;}function step(n){return n;}function initialize(){}}";
    // Access changes are a separate unsupported lookup profile.
    assert!(compile_sources(&[leaf, root]).is_err());
    let leaf = leaf.replace("function step", "protected function step");
    for sources in [vec![leaf.as_str(), root], vec![root, leaf.as_str()]] {
        common::pipeline(checking::compile_sources(&sources).unwrap());
    }
    for (base, child) in [
        (
            "(n as Toybox.Lang.Number) as Toybox.Lang.Number",
            "(n) as Toybox.Lang.Number",
        ),
        (
            "(n as Toybox.Lang.Number) as Toybox.Lang.Number",
            "(n as Toybox.Lang.Number)",
        ),
        ("(n)", "(n as Toybox.Lang.Number) as Toybox.Lang.Number"),
        (
            "(n as Toybox.Lang.Number)",
            "(n as Toybox.Lang.Number) as Toybox.Lang.Number",
        ),
        (
            "(n) as Toybox.Lang.Number",
            "(n as Toybox.Lang.Number) as Toybox.Lang.Number",
        ),
    ] {
        let source = format!(
            "class Root{{function value{base}{{return n;}}}}class Leaf extends Root{{function value{child}{{return n;}}}}function f(x as Leaf){{return x.value(1);}}"
        );
        common::pipeline(compile_sources(&[&source]).unwrap());
    }
}

#[test]
fn access_and_override_contracts_are_checked_at_the_declaring_owner() {
    let valid = "class Root{private function secret() as Toybox.Lang.Number{return 1;}public function read() as Toybox.Lang.Number{return secret();}protected function step() as Toybox.Lang.Number{return 2;}}class Leaf extends Root{private function secret() as Toybox.Lang.String{return \"leaf\";}function readStep() as Toybox.Lang.Number{return step();}}function f(x as Leaf){return x.read();}";
    common::pipeline(checking::compile_sources(&[valid]).unwrap());
    for source in [
        valid.replace("return step();", "return self.secret();"),
        valid.replace("return x.read();", "return x.step();"),
        valid.replace("return x.read();", "return x.secret();"),
        "class Root{private function secret(){return 1;}}class Leaf extends Root{function f(){return secret();}}".into(),
        "class Root{function f(n as Toybox.Lang.Number) as Toybox.Lang.Number{return n;}}class Leaf extends Root{function f(){return 1;}}".into(),
        "class Root{function f(n as Toybox.Lang.Number) as Toybox.Lang.Number{return n;}}class Leaf extends Root{function f(n as Toybox.Lang.Object) as Toybox.Lang.Number{return 1;}}".into(),
        "class Root{function f() as Toybox.Lang.Number{return 1;}}class Leaf extends Root{function f() as Toybox.Lang.String{return \"bad\";}}".into(),
        "class Root{function f(){return 1;}}class Leaf extends Root{private function f(){return 2;}}".into(),
        "class Root{function f(){return 1;}}class Leaf extends Root{static function f(){return 2;}}".into(),
        "class Root{function f(){return 1;}}class Leaf extends Root{var f=1;}".into(),
        "class Root{var f=1;}class Leaf extends Root{function f(){return 1;}}".into(),
        "class Root{static var n=1;}class Leaf extends Root{}function f(){return new Leaf();}".into(),
    ] {
        assert!(checking::compile_sources(&[&source]).is_err(), "{source}");
    }
}

#[test]
fn inherited_comparator_and_bound_method_use_parent_contracts() {
    let source = "using Toybox.Lang;class Root{function compare(a as Lang.Object,b as Lang.Object) as Lang.Number{return 0;}function value(n as Lang.Number) as Lang.Number{return n;}}class Leaf extends Root{}function f(x as Leaf){var a=[1,2];a.sort(x);var bound=x.method(:value);return bound.invoke(1);}";
    common::pipeline(checking::compile_sources(&[source]).unwrap());
}

#[test]
fn virtual_effects_include_overrides_through_parent_and_self_calls() {
    let classes = "class Root{function value() as Toybox.Lang.Number{return 1;}function forward() as Toybox.Lang.Number{return value();}}class Leaf extends Root{function value() as Toybox.Lang.Number{Toybox.System.exit();return 2;}}";
    for call in ["x.value();", "x.forward();"] {
        let source = format!(
            "{classes}class Holder{{var target as Root;function initialize(x as Root){{{call}self.target=x;}}}}"
        );
        let error = compile_sources(&[&source]).unwrap_err().to_string();
        assert!(error.contains("shutdown"), "{error}");
        let safe = source.replace(
            &format!("{call}self.target=x;"),
            &format!("self.target=x;{call}"),
        );
        common::pipeline(compile_sources(&[&safe]).unwrap());
    }
    let comparator = "using Toybox.Lang;class Root{function compare(a as Lang.Object,b as Lang.Object) as Lang.Number{return 0;}}class Leaf extends Root{function compare(a,b){Toybox.System.exit();return 0;}}class Holder{var target as Root;function initialize(c as Root){var a=[1,2];a.sort(c);self.target=c;}}";
    let error = compile_sources(&[comparator]).unwrap_err().to_string();
    assert!(error.contains("shutdown"), "{error}");
}

#[test]
fn saved_overrides_validate_contracts_even_when_the_body_ignores_its_input() {
    let source = "class Root{function value(n as Toybox.Lang.Number) as Toybox.Lang.Number{return 1;}}class Leaf extends Root{function value(n){return 2;}}";
    let text = compile_sources(&[source]).unwrap().to_ir_text();
    let (root, leaf) = text.split_once(".class Leaf ").unwrap();
    for changed in [
        leaf.replace(
            ".function value parameters=1 receiver=instance",
            ".function value parameters=1 parameter-types=string receiver=instance",
        ),
        leaf.replacen(
            "Lang.Object parent=Root\n",
            "Lang.Object parent=Root\n.visibility value protected\n",
            1,
        ),
    ] {
        assert_ne!(changed, leaf);
        assert!(parse_unit_ir(&format!("{root}.class Leaf {changed}")).is_err());
    }
}
