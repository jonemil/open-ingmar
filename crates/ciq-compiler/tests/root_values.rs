use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;

#[test]
fn root_values_resolve_to_existing_ir_and_complete_application_stages() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-root-values/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(!text.contains("$."));
    assert!(text.contains(".read-shared"));
    assert!(text.contains("Names.Child VALUE"));
    assert!(text.contains(".begin-shared"));
    assert!(text.contains("Toybox.Lang NUMBER_FORMAT_UINT8"));
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x5a1e3921a67e49cdb267cb392cad299fu128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
}

#[test]
fn roots_bypass_value_namespace_alias_and_type_shadowing() {
    for source in [
        "var X=1;module M{var X=2;function f(X as Toybox.Lang.Number){return $.X;}}",
        "module A{module B{var X=3;}}module Other{module A{module B{var X=9;}}function f(A as Toybox.Lang.Number){return $.A.B.X;}}",
        "using Other as A;module A{module B{var X=3;}}module Other{}function f(){return $.A.B.X;}",
        "const X=1;class C{var X=2;function f(){return $.X.toString();}}",
        "var A as Toybox.Lang.Array<Toybox.Lang.Number>=[1];function f(){return $.A.size();}",
        "class C{var n=1;function f(){return n;}}var V as C=new C();function f(){return $.V.f();}",
        "class C{}typedef T as C;module M{class C{}typedef T as Toybox.Lang.Number;function f(x as $.T) as $.C{return x as $.C;}}",
        "enum E{A=1}module M{enum E{B=2}function f(x as $.E) as $.E{return x;}}",
        "module M{class C{}class Holder{function f(C as Toybox.Lang.Number) as $.M.C{return new $.M.C();}}}",
        "function f(){return $.Toybox.Lang.NUMBER_FORMAT_UINT8;}",
        "function f(flag as Toybox.Lang.Boolean){$.Toybox.System.println(flag?1:2);}",
        "function f(System as Toybox.Lang.Number){$.Toybox.System.println(System);}",
        "function f(x as $.Toybox.Lang.Number) as $.Toybox.Lang.Number{return x;}",
        "const A=[1];function f(){$.A[0]=2;return $.A[0];}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}")));
    }
}

#[test]
fn root_access_keeps_existing_mutability_signature_and_namespace_limits() {
    for source in [
        "function f(){return $;}",
        "class C{}function f(){var C=1;return new $.C();}",
        "var X=1;function f(){return $.missing;}",
        "const X=1;function f(){$.X=2;}",
        "const X=1;function f(){$.X++;}",
        "module M{const X=1;}function f(){$.M.X+=2;}",
        "module M{}function f(){return $.M;}",
        "function g(){return 1;}function f(){return $.g;}",
        "class C{}function f(){return $.C;}",
        "var X as Toybox.Lang.Number=1;function f(){$.X=true;}",
        "module M{function g(n as Toybox.Lang.Number){return n;}}function f(){return $.M.g();}",
        "using Toybox.Lang;function f(){return $.Lang.NUMBER_FORMAT_UINT8;}",
        "class C{}module M{class D{}function f(x as D) as $.C{return x;}}",
        "class C{}module M{function f() as $.Missing{return new C();}}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}
