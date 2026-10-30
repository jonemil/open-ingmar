use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;

#[test]
fn class_enums_preserve_shared_defaults_types_and_allocation_effects() {
    let sources = [
        include_str!("../../../examples/compiler-class-enums/app.mc"),
        include_str!("../../../examples/compiler-class-enums/types.mc"),
    ];
    let unit = compile_sources(&sources).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".field-enum ON enum(C.Kind=integer) 7"));
    assert!(text.contains(".field-enum CURRENT enum(C.Kind=integer) null"));
    assert!(text.contains(".read-class "));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x5dd5f2b708274779b3501baceacdb41bu128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    common::pipeline(compile_sources(&[sources[1], sources[0]]).unwrap());
    for forged in [
        text.replace(
            ".field-enum ON enum(C.Kind=integer) 7",
            ".field-enum ON integer 7",
        ),
        text.replace(
            ".field-enum ON enum(C.Kind=integer) 7",
            ".field-enum ON string 7",
        ),
        text.replace(".enum C Kind integer\n", ""),
        text.replace("C ON result=enum(C.Kind=integer)", "C ON result=integer"),
        text.replace(
            "C ON result=enum(C.Kind=integer)",
            "C missing result=enum(C.Kind=integer)",
        ),
        text.replace(
            ".field-enum ON enum(C.Kind=integer) 7",
            ".field-value ON enum(C.Kind=integer)",
        ),
    ] {
        assert_ne!(forged, text);
        assert!(parse_unit_ir(&forged).is_err());
    }
}

#[test]
fn class_enum_writes_and_namespaces_are_checked() {
    for source in [
        "class C{enum{A}function initialize(){self.A=1;}}",
        "class C{enum{A}function initialize(){}}function f(){C.A=1;}",
        "class C{enum{A}function initialize(){}}function f(){var c=new C();c.A++;}",
        "class C{enum{A}const A=1;function initialize(){}}",
        "class C{enum Kind{A}function initialize(){}}function f(){return C.Kind.A;}",
        "class C{enum Kind{A}function initialize(){}}function f(){return new C.Kind();}",
        "class C{const A=1;function initialize(){}}function f(){return C.A;}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    for source in [
        "class C{enum{A}function initialize(){}}function f(C as C){return C.A;}",
        "class C{enum Kind{A,B}function initialize(){}function f() as Kind{return A;}}",
        "class C{enum{A}function initialize(){}}function f(){var C=new C();return C.A;}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap());
    }
}
