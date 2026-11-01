use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;

#[test]
fn module_classes_preserve_qualified_identity_and_complete_pipeline() {
    let sources = [
        include_str!("../../../examples/compiler-module-classes/app.mc"),
        include_str!("../../../examples/compiler-module-classes/later.mc"),
    ];
    let unit = compile_sources(&sources).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".class A.B.Token Lang.Object"));
    assert!(text.contains("source(A.B.Token@Lang.Object)"));
    assert!(text.contains("lookup=lexical"));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0xb67c1691c9a5422a9d3202a0c016870cu128.to_be_bytes(),
            entry: "Apps.MinimalConsoleApp.onStart".into(),
        },
    );
    common::pipeline(compile_sources(&[sources[1], sources[0]]).unwrap());
    assert!(parse_unit_ir(&text.replace("source(A.B.Token@", "source(Missing.Token@")).is_err());
}

#[test]
fn module_class_contracts_and_names_reject_invalid_claims() {
    for source in [
        "module A{class Token{}class Token{}}function f(){}",
        "module A{class Token{}module Token{}}function f(){}",
        "module A{class Token{}function Token(){}}",
        "module A{class Token{}const Token=1;}function f(){}",
        "module A{class Token{}}module B{class Token{}}function f(t as A.Token) as B.Token{return t;}",
        "module A{class Token{}}function f(){var A=1;return new A.Token();}",
        "module A{class Token{}}function f(){return new Missing.Token();}",
        "module A{class Token{}function f(value){var Token=1;return value as Token;}}",
        "module A{class Token{}}function f(value){var A=1;return value as A.Token;}",
        "module A{class Token{}function f(){var Token=1;return new Token();}}",
        "module A{class Token{}function f(value){var Token=1;return value instanceof Token;}}",
        "module Outer{class Token{}}module A{module Outer{}function f(){return new Outer.Token();}}",
        "module A{class Token{class Inner{}}}function f(){}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}

#[test]
fn lexical_class_types_work_in_fields_containers_and_callbacks() {
    for source in [
        "using Toybox.Lang;module A{class Token{}class Box{var token as Token or Null;function initialize(){token=new Token();}function get() as Token or Null{return token;}}}function f() as A.Box{return new A.Box();}",
        "using Toybox.Lang;module A{class Token{}typedef Tokens as Lang.Array<Token>;function f() as Tokens{return [new Token()];}}",
        "module A{class Token{}module B{function f() as Token{return new Token();}}}",
        "module A{class Token{}class Box{function make() as Token{return new Token();}}}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}")));
    }
}
