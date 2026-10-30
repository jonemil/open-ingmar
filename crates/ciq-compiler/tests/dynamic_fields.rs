use ciq_compiler::{compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn dynamic_fields_keep_metadata_defaults_and_the_complete_pipeline() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-dynamic-fields/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".field-dynamic-number value 1"));
    assert!(text.contains(".field-dynamic-symbol selector :start"));
    assert!(text.contains(".field-initialized computed any"));
    assert!(text.contains(".field-initialized object any"));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x1243d85fd10c457f8a9e6f23a7b0d981u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    for bad in [
        text.replace(
            ".field-dynamic-number value 1",
            ".field-dynamic-number value 2147483648",
        ),
        text.replace(
            ".field-dynamic-symbol selector :start",
            ".field-dynamic-symbol selector start",
        ),
        text.replace(
            ".field-dynamic-symbol selector :start",
            ".field-dynamic-symbol selector :bad-name",
        ),
        text.replace(
            ".field-initialized object any",
            ".field-initialized object void",
        ),
        text.replace("value result=any", "value result=integer"),
    ] {
        assert!(bad != text);
        assert!(parse_unit_ir(&bad).is_err());
    }
}

#[test]
fn initializers_do_not_prove_future_source_or_structural_member_contracts() {
    for source in [
        "class C{var X=1;function f(){self.X=\"text\";return self.X.length();}}",
        "class C{var X=:key;function f(){self.X=[1];return self.X.size();}}",
        "class C{var X=[1];function f(){self.X=null;self.X=\"text\";}}",
        "class C{var X=\"text\";function f(){self.X=[\"new\"];}}",
        "class C{var X=1;var Y=self.X;function f(){self.Y=:symbol;}}",
        "class C{var X as Toybox.Lang.Number=1;function f(){return self.X+1;}}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}")));
    }
    for source in [
        "class C{var X as Toybox.Lang.Number=1;function f(){self.X=\"bad\";}}",
        "class C{var X as Toybox.Lang.Symbol=:key;function f(){self.X=1;}}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    for source in [
        "class Token{function value(){return 41;}}class C{var X=new Token();function f(){return self.X.value();}}",
        "typedef T as interface{var X as Toybox.Lang.Number;};class C{var X=1;}function f(x as T){}function g(){f(new C());}",
    ] {
        assert!(checking::compile_sources(&[source]).is_err(), "{source}");
    }
}
