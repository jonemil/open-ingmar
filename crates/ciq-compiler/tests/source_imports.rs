use ciq_compiler::compile_sources;
mod common;

#[test]
fn source_imports_preserve_canonical_owners_across_all_stages() {
    let sources = [
        include_str!("../../../examples/compiler-source-imports/app.mc"),
        include_str!("../../../examples/compiler-source-imports/types.mc"),
    ];
    let unit = compile_sources(&sources).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains("source(Library.Models.Token@Lang.Object)"));
    assert!(!text.contains(".module M\n"));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0xb67c1691c9a5422a9d3202a0c016870cu128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    common::pipeline(compile_sources(&[sources[1], sources[0]]).unwrap());
}

const TYPES: &str = "using Toybox.Lang;module Lib{module Inner{typedef Num as Lang.Number;enum Kind{ONE=1}class Token{}function f(){return 1;}}}module Other{class Token{}}";

#[test]
fn imports_resolve_types_lazily_and_respect_declaration_scope() {
    for source in [
        "import Lib.Inner;function f(n as Num) as Token{return new Token();}",
        "using Lib.Inner as L;function f(){var L=1;return L.toString();}",
        "using Lib.Inner as L;function f(L as Toybox.Lang.Number){return L.toString();}",
        "using Lib.Inner as L;function f(t as L.Token){var L=1;return t as L.Token;}",
        "using Lib.Inner as L;class Box{var ONE as Toybox.Lang.Number=0;}function f(){var L=new Box();L.ONE=2;L.ONE++;return L.ONE;}",
        "using Lib.Inner as L;typedef T as L.Token;function f(t as T) as T{return t;}",
        "import Lib.Inner;import Other;function f(){return 1;}",
        "import Lib.Inner;class Token{}function f() as Token{return new Token();}",
        "module Outer{using Lib.Inner as L;module Lib{module Inner{class Token{}}}function f() as L.Token{return new L.Token();}}",
    ] {
        common::pipeline(
            compile_sources(&[source, TYPES]).unwrap_or_else(|e| panic!("{source}: {e}")),
        );
    }
    let owner = "module A{using Lib.Inner as L;function f(){return L.f();}}";
    assert!(compile_sources(&[owner, "function g(){return L.f();}", TYPES]).is_err());
    assert!(compile_sources(&[owner, "module A{function g(){return L.f();}}", TYPES]).is_err());
}

#[test]
fn source_imports_reject_ambiguous_types_invalid_targets_and_binding_conflicts() {
    assert!(compile_sources(&["using Missing as L;function f(){}"]).is_err());
    assert!(compile_sources(&["module A{using Missing as L;}function f(){}"]).is_err());
    for source in [
        "import Lib.Inner;import Other;function f() as Token{return new Token();}",
        "using Lib.Inner as L;using Other as L;function f(){}",
        "using Toybox.System as L;using Lib.Inner as L;function f(){}",
        "using Lib.Inner as L;using Toybox.System as L;function f(){}",
        "using Lib.Inner as Number;import Toybox.Lang;function f(){}",
        "import Toybox.Lang;using Lib.Inner as Number;function f(){}",
        "using Missing as L;function f(){}",
        "using Lib.Inner as L;function f(){var L=1;return new L.Token();}",
        "using Lib.Inner as L;function f(t as L.Token){var L=1;return t instanceof L.Token;}",
        "module Scope{using Lib.Inner as L;module Lib{}function f(){}}",
        "using Lib.Inner.Token as T;function f(){}",
        "import Lib.Inner.Token;function f(){}",
        "import Lib.Inner;function g(){return f();}",
        "using Lib.Inner;function g() as Token{return new Token();}",
    ] {
        assert!(compile_sources(&[source, TYPES]).is_err(), "{source}");
    }
}
