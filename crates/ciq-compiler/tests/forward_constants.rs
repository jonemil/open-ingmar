use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;
#[test]
fn forward_constants_preserve_complete_pipeline_and_initializer_order() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-forward-constants/app.mc"
    )])
    .unwrap();
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x3170f18fda334595aee207e933ec3c17u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}
#[test]
fn field_contract_inference_is_independent_of_file_order() {
    let a = "using Toybox.Lang;const ROOT=new Later().VALUE; class First{const VALUE=new Later().VALUE; function get() as Lang.String{return VALUE;}} function f() as Lang.String{return ROOT;}";
    let b = "class Later{const VALUE=\"later\";}";
    for sources in [[a, b], [b, a]] {
        common::pipeline(compile_sources(&sources).unwrap());
    }
    // A known later constant shadows a global immediate rather than inheriting
    // its type as a placeholder while its own declaration is unresolved.
    common::pipeline(compile_sources(&["const LATE=1;class C{const EARLY=LATE;const LATE=\"class\";function get() as Toybox.Lang.String{return EARLY;}}"]).unwrap());
}
#[test]
fn runtime_reads_and_cycles_are_not_fixed_by_reordering_effects() {
    for source in [
        "class C{const A=B;const B=[1];}",
        "class C{const A=B;const B=A;}",
        "class C{const A=B;const B=C;const C=\"late\";}",
        "const A=B;const B=A;function f(){}",
        "module M{const A=B;const B=A;}",
        "class C{const A=new D().B;}class D{const B=new C().A;}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}

#[test]
fn immediate_array_elements_are_literal_defaults_without_crossing_stored_shadows() {
    common::pipeline(
        compile_sources(&["const A=B;const B=[-1,1+2];function f(){return A;}"]).unwrap(),
    );
    let unit = compile_sources(&[
        "const X=1;module M{const A=B;const B=[X];const X=\"stored\";}function f(){}",
    ])
    .unwrap();
    // The stored module String shadows global X, so B is evaluated at runtime;
    // A snapshots B's initial Null rather than a literal Array default.
    assert!(unit.to_ir_text().contains(".shared-constant M A null"));
    common::pipeline(unit);
}
