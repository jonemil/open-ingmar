use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;

const CAPTURE: &str = "ciq-unit\n.owner global\nciq-ir\n.function f\n.begin-class &0 C N\n.integer %0 1\n.write-shared &0 %0\n.return-void\n.end\n.class C Lang.Object\n.field-static-variable N integer 0\nciq-ir\n.function initialize receiver=instance\n.return-void\n.end\n.end-class\n";

#[test]
fn mutable_static_fields_round_trip_with_the_complete_application() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-static-fields/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".field-static-variable VALUE integer static"));
    assert!(text.contains(".field-static-variable DYNAMIC any 2"));
    assert!(text.contains(".begin-class"));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0xb67c1691c9a5422a9d3202a0c016870cu128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    for sources in [
        vec!["class C{static var N as Toybox.Lang.Number=0;}function f(){C.N=1;}"],
        vec![
            "module M{class C{static var N as Toybox.Lang.Number=0;}}",
            "using M as A;function f(){A.C.N+=true?1:2;$.M.C.N++;}",
        ],
        vec!["class C{static var N as Toybox.Lang.Number=0;function f(){N+=1;self.N++;}}"],
        vec!["class C{static var N=0,S=\"ok\";}function f(){C.N=\"now a string\";C.S=2;}"],
        vec!["class C{static var X;}function f(){C.X=7;}"],
        vec!["class C{static var N=0;}function f(C as Toybox.Lang.Number){$.C.N=2;}"],
        vec!["class C{static var NEXT as C or Null;}function f(){C.NEXT=new C();}"],
        vec![
            "using Toybox.Lang;class C{static var M as Lang.Method or Null;}function f(){C.M=new Lang.Method(1,:abs);}",
        ],
        vec![
            "using Toybox.Lang;class C{static var B as Lang.ByteArray=[1]b;}function f(){C.B=[2]b;}",
        ],
    ] {
        common::pipeline(compile_sources(&sources).unwrap_or_else(|e| panic!("{sources:?}: {e}")));
    }
}

#[test]
fn static_writes_check_contracts_mutability_scope_and_readiness() {
    for source in [
        "static var N=1;",
        "module M{static var N=1;}",
        "class C{static var N as Toybox.Lang.Number;}",
        "class C{static var N as Toybox.Lang.Number=0;}function f(){C.N=\"bad\";}",
        "using Toybox.Lang;class C{static var A as Lang.Array<Lang.Number>=[1];}function f(){C.A=[\"bad\"];}",
        "class C{static const N=0;}function f(){C.N=1;}",
        "class C{var N=0;}function f(){C.N=1;}",
        "class C{enum{N}}function f(){C.N++;}",
        "class C{static var N=0;var N=1;}",
        "class C{static var N=0;}function f(C as Toybox.Lang.Number){C.N=1;}",
        "class C{var N=0;static var OTHER=N;}",
        "using Toybox.Lang;class C{static var PAIR as [Lang.Number] or Null;function f(other as C){if(self.PAIR!=null){other.PAIR=null;self.PAIR[0]=7;}}}",
        "using Toybox.Lang;class C{static var PAIR as [Lang.Number] or Null;function f(){if(self.PAIR!=null){C.PAIR=null;self.PAIR[0]=7;}}}",
        "using Toybox.Lang;class C{static var A as Lang.Number=change();static var B as Lang.Number=value();}function value() as Lang.Number{return 1;}function change() as Lang.Number{C.B=9;return 0;}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}

#[test]
fn class_captures_use_checked_shared_completion_and_reject_forged_ir() {
    common::pipeline(parse_unit_ir(CAPTURE).unwrap());
    for forged in [
        CAPTURE.replace(".begin-class &0 C N\n", ""),
        CAPTURE.replace(".write-shared &0 %0", ".write-shared &9 %0"),
        CAPTURE.replace(
            ".write-shared &0 %0",
            ".write-shared &0 %0\n.write-shared &0 %0",
        ),
        CAPTURE.replace(".write-shared &0 %0\n", ""),
        CAPTURE.replace(".begin-class &0 C N", ".begin-class &0 Missing N"),
        CAPTURE.replace(".begin-class &0 C N", ".begin-class &0 C Missing"),
        CAPTURE.replace(".integer %0 1", ".string %0 \"bad\""),
        CAPTURE.replace(
            ".field-static-variable N integer 0",
            ".field-static-constant N integer 0",
        ),
        CAPTURE.replace(".field-static-variable N integer 0", ".field-number N 0"),
        CAPTURE.replace(
            ".field-static-variable N integer 0",
            ".field-static-variable N integer null",
        ),
    ] {
        assert_ne!(CAPTURE, forged);
        assert!(parse_unit_ir(&forged).is_err(), "{forged}");
    }
    let reordered = CAPTURE.replace(".begin-class &0 C N\n.integer %0 1\n.write-shared &0 %0\n.return-void", ".block ^0\n.jump ^1\n.block ^2\n.integer %0 1\n.write-shared &0 %0\n.jump ^3\n.block ^1\n.begin-class &0 C N\n.jump ^2\n.block ^3\n.return-void");
    common::pipeline(parse_unit_ir(&reordered).unwrap());
    assert!(
        parse_unit_ir(&reordered.replace(".begin-class &0 C N", ".begin-class &0 Missing N"))
            .is_err()
    );
}

#[test]
fn dynamic_class_stores_obey_the_checking_policy() {
    let source = "class C{static var N as Toybox.Lang.Number=0;}function f(value){C.N=value;}";
    let output = ciq_compiler::compile_sources_with_options(
        &[source],
        ciq_compiler::CompileOptions {
            type_check: ciq_compiler::TypeCheckLevel::Informative,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        output
            .warnings
            .iter()
            .any(|w| w.kind == ciq_compiler::TypeDiagnosticKind::AmbiguousBoundary)
    );
    assert!(
        ciq_compiler::compile_sources_with_options(
            &[source],
            ciq_compiler::CompileOptions {
                type_check: ciq_compiler::TypeCheckLevel::Strict,
                ..Default::default()
            }
        )
        .is_err()
    );
}
