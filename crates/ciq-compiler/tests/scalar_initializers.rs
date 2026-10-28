use ciq_compiler::{compile_sources, compile_sources_with_api, parse_unit_ir};
mod common;

#[test]
fn calculated_scalars_keep_ordered_reads_native_types_and_all_saved_stages() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-scalar-initializers/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    for owner in ["$", "M"] {
        for field in [
            "EARLY_NUMBER",
            "EARLY_FLOAT",
            "EARLY_ALIAS",
            "SELF_READ",
            "CYCLE_A",
            "CYCLE_B",
            "EARLY_FORMAT",
        ] {
            assert!(text.contains(&format!(".shared-constant {owner} {field} null")));
        }
        assert!(text.contains(&format!(".shared-constant {owner} FORMAT number-format")));
        assert!(text.contains(&format!(".shared-constant {owner} ENDIAN endian")));
    }
    for operation in [
        ".add ",
        ".subtract ",
        ".multiply ",
        ".divide ",
        ".less ",
        "Toybox.Lang NUMBER_FORMAT_UINT16",
    ] {
        assert!(text.contains(operation), "{operation}");
    }
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x2af6def0d6bc4e6eacaf521ae0446721u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    let forged = text.replace(
        ".shared-constant $ FORMAT number-format",
        ".shared-constant $ FORMAT endian",
    );
    assert!(parse_unit_ir(&forged).is_err());
}

#[test]
fn native_constant_aliases_keep_api_and_argument_checks() {
    let low = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(5, 2, 0),
        ciq_compiler::api::ApiLevel(3, 0, 0),
    )
    .unwrap();
    for source in [
        "using Toybox.Lang;const UNUSED=Lang.NUMBER_FORMAT_UINT16;function f(){}",
        "using Toybox.Lang;const UNUSED=Lang.ENDIAN_BIG==null;function f(){}",
    ] {
        assert!(compile_sources_with_api(&[source], low).is_err());
        common::pipeline(compile_sources(&[source]).unwrap());
    }
    let source = "using Toybox.Lang;const FORMAT=Lang.NUMBER_FORMAT_UINT16;function f(){var b=[0,0]b;b.encodeNumber(1,FORMAT,{});}";
    common::pipeline(compile_sources(&[source]).unwrap());
    for source in [
        "using Toybox.Lang;const FORMAT=Lang.ENDIAN_BIG;function f(){var b=[0,0]b;b.encodeNumber(1,FORMAT,{});}",
        "const FORMAT=4;function f(){var b=[0,0]b;b.encodeNumber(1,FORMAT,{});}",
        "const BAD=1.0f & 2.0f;function f(){}",
        "const BAD=:a < :b;function f(){}",
        "const BAD=true + false;function f(){}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}
