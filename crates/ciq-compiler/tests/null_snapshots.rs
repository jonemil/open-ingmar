use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;
const PROGRAM: &str = "module T{function make() as Toybox.Lang.Array<Toybox.Lang.Number>{return [7];}} const EARLY=LATE;const LATE=T.make();const AFTER=LATE;function f(){Toybox.System.println(LATE==null);}";
#[test]
fn null_snapshots_keep_read_instructions_and_all_saved_stages() {
    let unit = compile_sources(&[
        include_str!("../../../examples/compiler-null-snapshots/app.mc"),
        include_str!("../../../examples/compiler-null-snapshots/later.mc"),
    ])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".shared-constant $ EARLY null"));
    assert!(text.contains(".shared-constant M EARLY null"));
    assert!(text.contains(".read-shared %0 $ LATE result=null"));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x8744df3eac9d4803bd9e5b11fd417221u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    let early = unit
        .declarations()
        .iter()
        .find(|d| d.function.name() == "__ciq_initialize_shared_EARLY" && d.module.is_none())
        .unwrap();
    assert!(
        early
            .function
            .lower()
            .to_text()
            .contains("getmv @__ciq_source_globals @__ciq_field_LATE")
    );
}
#[test]
fn saved_read_views_must_match_initialization_state() {
    let text = compile_sources(&[PROGRAM]).unwrap().to_ir_text();
    let forged = text.replace(
        ".read-shared %0 $ LATE result=null",
        ".read-shared %0 $ LATE result=number-array",
    );
    assert_ne!(forged, text);
    assert!(
        parse_unit_ir(&forged)
            .unwrap_err()
            .message
            .contains("current slot state")
    );
    for name in ["__ciq_initialize_shared_AFTER", "f"] {
        let start = text.find(&format!(".function {name}")).unwrap();
        let end = start + text[start..].find(".end\n").unwrap();
        let edited = text[start..end].replace("LATE result=number-array", "LATE result=null");
        assert_ne!(&text[start..end], edited);
        let mut forged = text.clone();
        forged.replace_range(start..end, &edited);
        assert!(parse_unit_ir(&forged).is_err(), "{name}");
    }
    let undeclared = text.replace(
        ".read-shared %0 $ LATE result=null",
        ".read-shared %0 $ MISSING result=null",
    );
    assert!(parse_unit_ir(&undeclared).is_err());
}
#[test]
fn null_is_not_a_placeholder_for_nonnullable_arguments_or_other_owners() {
    for source in [
        "module T{function take(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Array<Toybox.Lang.Number>{return a;}function make() as Toybox.Lang.Array<Toybox.Lang.Number>{return [1];}}const EARLY=T.take(LATE);const LATE=T.make();function f(){}",
        "const EARLY=LATE.size();module T{function make() as Toybox.Lang.Array<Toybox.Lang.Number>{return [1];}}const LATE=T.make();function f(){}",
        "const EARLY=LATE[0];const LATE=[[1]];function f(){}",
        "const X=M.get();module M{const Y=f();function get() as Toybox.Lang.Array<Toybox.Lang.Number>{return Y;}}function f() as Toybox.Lang.Array<Toybox.Lang.Number>{return X;}",
        "class C{const EARLY=LATE;const LATE={:value=>1};}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}
