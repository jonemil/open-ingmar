use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;

#[test]
fn enum_value_types_and_defaults_keep_all_stages() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-enum-values/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".field-enum TEXT string static"));
    assert!(text.contains(".field-enum TOKEN source(Token@Lang.Object) null"));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x2fcb5a7c8c334a009e948607ddc8fdc5u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    for forged in [
        text.replace(
            ".field-enum TEXT string static",
            ".field-enum TEXT string 3",
        ),
        text.replace(
            ".field-enum TEXT string static",
            ".field-enum TEXT string null",
        ),
        text.replace(
            ".field-enum TEXT string static",
            ".field-enum TEXT string invalid",
        ),
        text.replace(
            ".field-enum TEXT string static",
            ".field-enum TEXT void static",
        ),
        text.replace("__ciq_initialize_enum_defaults", "arbitrary_default_method"),
    ] {
        assert_ne!(forged, text);
        assert!(parse_unit_ir(&forged).is_err());
    }
}

#[test]
fn anonymous_members_keep_types_and_named_non_number_values_remain_rejected() {
    for source in [
        "using Toybox.Lang;enum{F=1.5f}function f() as Lang.Number{return F;}",
        "using Toybox.Lang;enum Kind{F=1.5f}function f() as Lang.Float{return F;}",
        "using Toybox.Lang;enum Kind{S=\"text\"}function f(){return S.length();}",
        "class C{enum{A=1.5f}function initialize(){self.A=2.5f;}}",
        "class C{enum{A=1.5f}function initialize(){}}function __ciq_initialize_enum_defaults(){}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    for source in [
        "using Toybox.Lang;enum{F=1.5f,N}function f() as Lang.Float{return F;}",
        "using Toybox.Lang;enum{S=\"text\"}function f() as Lang.String{return S;}",
        "using Toybox.Lang;enum{N=null}function f(){return N;}",
        "using Toybox.Lang;class C{enum{A=1.5f}function initialize(){}}function f() as Lang.Float{return C.A;}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap());
    }
}
