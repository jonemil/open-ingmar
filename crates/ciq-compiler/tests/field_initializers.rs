use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;

#[test]
fn declaration_initializers_preserve_types_owners_and_complete_pipeline() {
    let sources = [
        include_str!("../../../examples/compiler-field-initializers/app.mc"),
        include_str!("../../../examples/compiler-field-initializers/box.mc"),
    ];
    let unit = compile_sources(&sources).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".field-initialized text any"));
    assert!(text.contains(".field-initialized items number-array"));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x8022a5c1d89449c18a1bf306ca0bbc65u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    common::pipeline(compile_sources(&[sources[1], sources[0]]).unwrap());
    for bad in [
        text.replace(".field-initialized text any", ".field-constant text any"),
        text.replace(
            ".field-initialized items number-array",
            ".field-initialized items string",
        ),
        text.replace(".field-initialized text any\n", ""),
        text.lines()
            .filter(|line| !line.starts_with(".assign-self text "))
            .collect::<Vec<_>>()
            .join("\n"),
    ] {
        assert!(bad != text);
        assert!(parse_unit_ir(&bad).is_err());
    }
}

#[test]
fn declarations_share_inference_context_and_initialization_readiness() {
    for (ty, value) in [
        ("Number", "1"),
        ("Long", "1l"),
        ("Float", "1.5f"),
        ("Double", "2.5d"),
        ("String", "\"value\""),
        ("Char", "'Q'"),
        ("Boolean", "true"),
        ("Numeric", "1"),
        ("Integer", "1"),
        ("Decimal", "1.5f"),
    ] {
        for nullable in ["", " or Null"] {
            let source = format!("class C{{var value as Toybox.Lang.{ty}{nullable}={value};}}");
            common::pipeline(
                compile_sources(&[&source]).unwrap_or_else(|e| panic!("{source}: {e}")),
            );
        }
    }
    for source in [
        "class C{var A=1+2;var B=self.A+1;function f(){self.A=4;return self.B;}}",
        "class C{var A=self.B.length();var B as Toybox.Lang.String=\"later\";}",
        "class C{var A as Toybox.Lang.Number=1;var B as Toybox.Lang.Symbol=:key;}",
        "class C{var A as Toybox.Lang.Number or Null=null;var B as Toybox.Lang.String=\"value\";}",
        "class C{var A=true?1:2;var B=true&&false;const X=true?[1]:[2];}",
        "class Token{} class C{var A as Token=new Token();var B=new Token();function f(){self.B=new Token();}}",
        "class Token{}class C{var A as Token or Null=new Token();var B as Token or Null=(null);}",
        "enum Kind{A,B}class C{var K as Kind=A;function f(){self.K=B;}}",
        "class C{var A=[1,2];var B=[3,4]b;var D={\"x\"=>1};}",
        "class C{var A;var B=null;function f(){self.A=1;self.B=\"text\";}}",
        "class C{var A as Toybox.Lang.Method=1.method(:abs);}",
        "class C{var T as Toybox.Lang.Array<Toybox.Lang.Number> = [1];var W as Toybox.Lang.WeakReference=self.T.weak();}",
        "class C{var A as Toybox.Lang.Comparable or Null=1;var B as Toybox.Lang.Object=\"value\";}",
        "class C{var A as Toybox.Lang.Number or Toybox.Lang.String=1;var B as Toybox.Lang.Number or Toybox.Lang.String or Null=1;}",
        "class C{var A as Toybox.Lang.Array<Toybox.Lang.String> = [];var B as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.Number> = {};}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}")));
    }
    for source in [
        "class C{var A as Toybox.Lang.Number=\"bad\";}",
        "class C{var A as Toybox.Lang.Array<Toybox.Lang.Number> = [1];function f(){self.A=[\"wrong\"];}}",
        "class C{var A as Toybox.Lang.String=\"x\";function f(){self.A=1;}}",
        "class C{var A=self.B;var B=self.A;}",
        "class C{var A=self.B;var B=1+2;}",
        "class C{var A as Toybox.Lang.Number;var B=self.A;function initialize(){self.A=1;}}",
        "class C{var A=self.f();function f() as Toybox.Lang.Number{return 1;}}",
        "function stop() as Toybox.Lang.Number{Toybox.System.exit();return 1;}class C{var A=stop();}",
        "class C{var A=1;var A=2;}",
        "class C{var A=\"x\";const A=\"y\";}",
        "class C{var A as Void=1;}",
        "class C{var A as Toybox.Lang.Decimal=1;}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}

#[test]
fn allocation_write_proofs_follow_cfg_instead_of_text_order() {
    let text = compile_sources(&["class C{var X=1+2;}"])
        .unwrap()
        .to_ir_text();
    let start = text.find(".function __ciq_initialize_fields").unwrap();
    let end = start + text[start..].find(".end").unwrap() + ".end".len();
    let prefix = ".function __ciq_initialize_fields receiver=instance\n.block ^0\n.boolean %0 true\n.branch %0 ^1 ^2\n.block ^1\n.integer %1 3\n.assign-self X %1\n.jump ^3\n.block ^2\n.integer %2 4\n.assign-self X %2\n.jump ^3\n.block ^3\n.return-void\n.end";
    let mut branched = text;
    branched.replace_range(start..end, prefix);
    common::pipeline(parse_unit_ir(&branched).unwrap());
    for bad in [
        branched.replace(".assign-self X %2\n", ".discard %2\n"),
        branched.replace(".jump ^3\n.block ^2", ".jump ^1\n.block ^2"),
        branched.replace(".assign-self X %1", ".assign-self missing %1"),
    ] {
        assert!(parse_unit_ir(&bad).is_err());
    }
}

#[test]
fn class_statics_cannot_read_fields_awaiting_instance_allocation() {
    let mut text = compile_sources(&["class C{enum{A=\"static\"}var X=\"instance\";}"])
        .unwrap()
        .to_ir_text();
    let start = text
        .find(".function __ciq_initialize_enum_defaults")
        .unwrap();
    let end = start + text[start..].find(".end").unwrap() + ".end".len();
    text.replace_range(start..end, ".function __ciq_initialize_enum_defaults receiver=instance\n.read-self %0 X result=string\n.assign-self A %0\n.return-void\n.end");
    let error = parse_unit_ir(&text).unwrap_err();
    assert!(
        error
            .message
            .contains("field read before required initialization"),
        "{error}"
    );
}
