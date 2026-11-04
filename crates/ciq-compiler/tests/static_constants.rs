use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;

#[test]
fn static_constants_keep_shared_storage_and_complete_saved_stages() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-static-constants/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".field-static-constant DEFAULT integer 7"));
    assert!(text.contains(".field-static-constant MESSAGE string static"));
    assert!(text.contains(".function __ciq_initialize_statics"));
    let prg = common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0xb67c1691c9a5422a9d3202a0c016870cu128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    // A statics initializer's lexical parent is its containing module, so a
    // same-module helper remains reachable while the static receiver is active.
    let records =
        ciq_prg::data::classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
            .unwrap();
    let classes = records
        .iter()
        .filter_map(|r| match r {
            ciq_prg::data::classes::Record::Class(c) => Some(c),
            _ => None,
        })
        .collect::<Vec<_>>();
    let definition = classes
        .iter()
        .find(|c| {
            c.statics.is_some()
                && classes
                    .iter()
                    .any(|module| module.module == c.parent && module.parent.is_some())
        })
        .unwrap();
    let static_id = definition.statics.unwrap();
    let static_offset = classes
        .iter()
        .flat_map(|c| &c.fields)
        .find(|f| f.key >> 8 == static_id && f.kind == 10)
        .unwrap()
        .value;
    let statics = classes
        .iter()
        .find(|c| c.offset == static_offset as usize)
        .unwrap();
    assert_eq!(statics.parent, definition.parent);
    for sources in [
        vec![
            "module M{class C{static const N=7;static const S=\"ok\";}}",
            "using M as A;function f(){return A.C.S;}",
        ],
        vec![
            "function f(){return $.M.C.N;}",
            "module M{class C{static const N=7;}}",
        ],
        vec!["class C{static const N=7,S=\"ok\";function f(){return N;}}"],
        vec!["class C{static const N=7;function f(){var c=new C();return c.N;}}"],
        vec![
            "using Toybox.Lang;class C{static const A as Lang.Array<Lang.Number>=[1];static const B as Lang.Array<Lang.Number>=A;}",
        ],
        vec!["class C{static const N=null;}"],
        vec!["class C{static const N;}"],
        vec!["class C{enum {A=7} static const B=A;}"],
        vec!["class C{static const N=7;}function f(C as Toybox.Lang.Number){return $.C.N;}"],
    ] {
        common::pipeline(compile_sources(&sources).unwrap_or_else(|e| panic!("{sources:?}: {e}")));
    }
}

#[test]
fn static_constants_reject_rebinding_invalid_scopes_and_instance_context() {
    for source in [
        "static const N=7;",
        "module M{static const N=7;}",
        "class C{static const N=7;function f(){N=9;}}",
        "class C{static const N=7;function f(){self.N++;}}",
        "class C{static const N=7;}function f(){C.N=9;}",
        "class C{static const N=7;}function f(){var c=new C();c.N=9;}",
        "class C{static const N=7;var N=9;}",
        "class C{static const N=7;function N(){}}",
        "class C{var n=7;static const N=n;}",
        "class C{function n(){return 7;}static const N=n();}",
        "class C{static const N=7;}function f(C as Toybox.Lang.Number){return C.N;}",
        "class C{static const N as Toybox.Lang.String=7;}",
        "class C{static const N as Toybox.Lang.Number;}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}

#[test]
fn saved_static_storage_cannot_forge_contracts_or_initialization() {
    let unit = compile_sources(&["class C{static const N=7;static const S=\"ok\";}"]).unwrap();
    let text = unit.to_ir_text();
    for forged in [
        text.replace(
            ".field-static-constant S string static",
            ".field-static-constant S string null",
        ),
        text.replace(
            ".field-static-constant N integer 7",
            ".field-static-constant N string 7",
        ),
        text.replace(".assign-self S %0", ".assign-self N %0"),
        text.replace(".assign-self S %0", ""),
        text.replace(
            ".field-static-constant S string static",
            ".field-static-constant S void static",
        ),
        text.replace(
            ".field-static-constant S string static",
            ".field-constant S string",
        ),
    ] {
        assert_ne!(forged, text);
        assert!(parse_unit_ir(&forged).is_err(), "{forged}");
    }
}

#[test]
fn statics_cannot_observe_unwritten_constants_through_calls_or_qualified_reads() {
    for source in [
        "using Toybox.Lang;class C{static const S as Lang.String=f();}function f() as Lang.String{return C.S;}",
        "using Toybox.Lang;class C{static const S as Lang.String=D.S;}class D{static const S as Lang.String=C.S;}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let text = compile_sources(&["class C{static const S=\"ok\";}"])
        .unwrap()
        .to_ir_text();
    let forged = text.replace(".string %0 \"ok\"", ".read-class %0 C S result=string");
    assert_ne!(forged, text);
    assert!(parse_unit_ir(&forged).is_err());
}
