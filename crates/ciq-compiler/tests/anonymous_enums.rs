use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;

#[test]
fn numbered_members_and_runtime_values_keep_all_saved_stages() {
    let sources = [
        include_str!("../../../examples/compiler-anonymous-enums/app.mc"),
        include_str!("../../../examples/compiler-anonymous-enums/later.mc"),
    ];
    let unit = compile_sources(&sources).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".shared-constant M AFTER_RUNTIME integer"));
    assert!(text.contains(".shared-constant $ GROUPED_EARLY null"));
    assert!(text.contains(".shared-constant $ ORDINARY_EARLY null"));
    assert!(text.contains(".integer %0 -2147483648"));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x2e0caf1206d441caa6fb00fdd46f0721u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    common::pipeline(compile_sources(&[sources[1], sources[0]]).unwrap());
    assert!(
        parse_unit_ir(&text.replace(".shared-constant M AFTER_RUNTIME integer\n", "")).is_err()
    );
}

#[test]
fn enum_scope_duplicates_readonly_writes_and_bounds_are_checked() {
    for source in [
        "enum{}function f(){}",
        "enum{A,,B}function f(){}",
        "enum{A=,B}function f(){}",
        "enum{A,A}function f(){}",
        "const A=1;enum{A}function f(){}",
        "enum{A}function A(){}",
        "module M{enum{A}enum{A}}function f(){}",
        "enum{A};function f(){}",
        "enum{A}function f(){A=1;}",
        "module M{enum{A}}function f(){M.A++;}",
        "enum{A=1.5f,B}function f(){}",
        "enum{A=missing}function f(){}",
        "function f(){enum{A}}",
        "enum Kind{A}function f(){}",
        "class C{enum{A}}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    for source in [
        "enum{A,B,}function f(){return B;}",
        "enum{A=1<2 ? 5:6,B}function f(){return B;}",
        "const EARLY=M.B;module M{enum{A=4,B}}function f(){return EARLY;}",
        "module M{enum{A}function f(){return self.A;}}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap());
    }
    let members = (0..257)
        .map(|n| format!("C{n}"))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!("enum{{{members}}}function f(){{}}");
    assert!(
        compile_sources(&[&source])
            .unwrap_err()
            .diagnostic
            .message
            .contains("256 constant")
    );
}
