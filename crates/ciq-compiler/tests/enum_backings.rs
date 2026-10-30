use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;

#[test]
fn enum_backings_survive_calls_containers_and_saved_boundaries() {
    let sources = [
        include_str!("../../../examples/compiler-enum-backings/app.mc"),
        include_str!("../../../examples/compiler-enum-backings/types.mc"),
    ];
    let unit = compile_sources(&sources).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".enum $ Floats float"));
    assert!(text.contains(".enum $ Texts string"));
    assert!(text.contains("enum(Floats=float)"));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x7392fa5b529049abbb3e85c6f6acd928u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    common::pipeline(compile_sources(&[sources[1], sources[0]]).unwrap());
    for forged in [
        text.replace(".enum $ Floats float", ".enum $ Floats integer"),
        text.replace("enum(Floats=float)", "enum(Floats=integer)"),
        text.replace(".enum $ Floats float\n", ""),
        text.replace("enum(Floats=float)", "enum(Floats=?)"),
    ] {
        assert_ne!(forged, text);
        assert!(parse_unit_ir(&forged).is_err());
    }
}

#[test]
fn nominal_boundaries_keep_native_backings_without_accepting_raw_values() {
    for source in [
        "using Toybox.Lang;enum Kind{F=1.5f}function f(v as Kind) as Lang.Number{return v;}",
        "enum Kind{S=\"text\"}function f(v as Kind){}function g(){f(\"text\");}",
        "enum A{A_VALUE=\"text\"}enum B{B_VALUE=\"text\"}function f(v as A){}function g(){f(B_VALUE);}",
        "using Toybox.Lang;enum Kind{S=\"text\"}function f(v as Lang.Array<Lang.String>){}function make() as Lang.Array<Kind>{return [S];}function g(){f(make());}",
        "enum A{VALUE=f()}function f() as A{return VALUE;}",
        "enum Kind{A=[1,2]}function f(){A[0]=\"bad\";}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    for source in [
        "enum Kind{A=[1,2]}function f(){A[0]=3;return A[1];}",
        "enum Kind{D={:x=>1}}function f(){D[:x]=3;return D[:x];}",
        "class Token{function initialize(){}function value(){return 4;}}enum Kind{ITEM=new Token()}function use(v as Kind){return v.value();}function f(){return use(ITEM);}",
        "using Toybox.Lang;enum Kind{F=1.5f}function f(v as Kind) as Lang.Float{return v;}",
        "enum Kind{S=\"text\",F=1.5f}function f(v as Kind){return v.length();}",
        "using Toybox.Lang;enum Kind{S=\"text\",F=1.5f}function f(v as Kind) as Lang.Float{return v;}",
        "using Toybox.Lang;enum Kind{A=1,B=Choice.choose(A)}module Choice{function choose(v as Kind) as Kind{return v;}}function f(){return B;}",
        "enum A{VALUE=M.VALUE}module M{enum Kind{VALUE=\"text\"}}function f(){return VALUE.length();}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}")));
    }
}

#[test]
fn unused_enum_aliases_resolve_with_api_and_saved_metadata_is_bounded() {
    let context = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(5, 2, 0),
        ciq_compiler::api::ApiLevel(5, 0, 0),
    )
    .unwrap();
    let unit=ciq_compiler::compile_sources_with_api(&["using Toybox.Lang;enum Kind{F=1.5f}typedef Unused as Lang.Array<Kind>;function f(){return F;}"],context).unwrap();
    common::pipeline(unit);
    let nested = format!("{}integer{}", "enum(E=".repeat(66), ")".repeat(66));
    let text =
        format!("ciq-ir\n.function f parameters=1 parameter-types={nested}\n.return-void\n.end\n");
    assert!(ciq_compiler::parse_function_ir(&text).is_err());
}
