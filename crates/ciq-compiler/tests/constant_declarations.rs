use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;

#[test]
fn annotated_lists_preserve_contracts_and_the_complete_application_pipeline() {
    let sources = [
        include_str!("../../../examples/compiler-constant-declarations/app.mc"),
        include_str!("../../../examples/compiler-constant-declarations/later.mc"),
    ];
    let unit = compile_sources(&sources).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".shared-constant $ OBJECT object"));
    assert!(text.contains(".shared-constant $ NUMBER integer"));
    assert!(text.contains(".shared-constant $ EMPTY number-array"));
    assert!(text.contains(".shared-constant M NONE nullable-integer"));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x43ae045dd18b4b90b84cb151e55a0721u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    common::pipeline(compile_sources(&[sources[1], sources[0]]).unwrap());
    let forged = text.replace(
        ".shared-constant $ OBJECT object",
        ".shared-constant $ OBJECT integer",
    );
    assert_ne!(forged, text);
    assert!(parse_unit_ir(&forged).is_err());
}

#[test]
fn types_defaults_and_list_boundaries_are_checked_per_binding() {
    for source in [
        "const X as Toybox.Lang.Number=\"bad\";function f(){}",
        "const X as Toybox.Lang.Number;function f(){}",
        "class C{const X as Toybox.Lang.String;}",
        "const X as Missing=1;function f(){}",
        "const X as Toybox.Lang.NumberFormat=4;function f(){}",
        "const X as Toybox.Lang.Float=1;function f(){}",
        "const X as Toybox.Lang.Object=7;function take(n as Toybox.Lang.Number){}function f(){take(X);}",
        "const A=1,A=2;function f(){}",
        "const A=1,;function f(){}",
        "const A=,B=2;function f(){}",
        "const A as=1;function f(){}",
        "const A=1,B=2;function f(){B=3;}",
        "module M{const A=1,B=2;}function f(){M.B++;}",
        "class C{const A=1,B=2;function f(){self.B=3;}}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    for source in [
        "const A=1,B,C=3;function f(){return B;}",
        "const A as Toybox.Lang.Array<Toybox.Lang.Number> = [], B=[1,2];function f(){A.add(3);return B;}",
        "const A as Toybox.Lang.Number or Null;function f(){return A;}",
        "module M{typedef N as Toybox.Lang.Number;const A as N=1,B as N=2;}function f(){return M.B;}",
        "const A as Toybox.Lang.Number=1<2 ? 3:4, B=5;function f(){return A+B;}",
        "const A=({:key=>1} as Toybox.Lang.Dictionary<Toybox.Lang.Symbol,Toybox.Lang.Number>), B=2;function f(){return A;}",
        "const A={:key=>1} as Toybox.Lang.Dictionary<Toybox.Lang.Symbol,Toybox.Lang.Number>, B=2;function f(){return A;}",
        "const A=([] as Toybox.Lang.Array<Toybox.Lang.Array<Toybox.Lang.Number>>), B=2;function f(){return A;}",
        "class Cell{var Array as Toybox.Lang.Number;function initialize(){self.Array=1;}}const A=(new Cell() as Cell).Array < 3, B=2;function f(){return A;}",
        "const Array=1;module Trace{function take(n as Toybox.Lang.Number,b as Toybox.Lang.Boolean) as Toybox.Lang.Number{return n;}}const A=Trace.take(1 as Toybox.Lang.Number,Array < 3), B=2;function f(){return A;}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap());
    }
    let mut source = String::from("const ");
    for n in 0..257 {
        if n != 0 {
            source.push(',');
        }
        source.push_str(&format!("C{n}={n}"));
    }
    source.push_str(";function f(){}");
    assert!(
        compile_sources(&[&source])
            .unwrap_err()
            .diagnostic
            .message
            .contains("256 constant")
    );
    let low = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(1, 0, 0),
        ciq_compiler::api::ApiLevel(1, 0, 0),
    )
    .unwrap();
    assert!(
        ciq_compiler::compile_sources_with_api(
            &["const UNUSED as Toybox.Lang.Char or Null;function f(){}"],
            low
        )
        .is_err()
    );
}
