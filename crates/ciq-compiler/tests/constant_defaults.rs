use ciq_compiler::compile_sources;
mod common;

#[test]
fn literal_defaults_and_runtime_expressions_preserve_the_application_pipeline() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-constant-defaults/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    for owner in ["$", "M"] {
        assert!(text.contains(&format!(
            ".shared-constant {owner} EARLY_LITERAL number-array"
        )));
        for name in [
            "EXPR",
            "REF",
            "BYTES",
            "TEXT",
            "WIDE",
            "EMPTY",
            "EMPTY_BYTES",
            "GROUPED",
            "SUM",
            "JOIN",
        ] {
            assert!(text.contains(&format!(".shared-constant {owner} EARLY_{name} null")));
        }
        assert!(!text.contains(&format!(".shared-constant {owner} EARLY_SIGNED null")));
    }
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0xc3907c415fdf40139a859dd4fc430721u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
}

#[test]
fn default_eligibility_depends_on_source_form_not_foldability() {
    for expression in [
        "[1+2]",
        "[N]",
        "[+N]",
        "[N]b",
        "[]",
        "[]b",
        "[~1]",
        "[!false]",
        "[-(1)]",
        "[-(-1)]",
        "[1l+2l]",
        "[\"a\"+\"b\"]",
    ] {
        let source =
            format!("const EARLY=VALUES;const VALUES={expression};const N=7;function f(){{}}");
        let unit = compile_sources(&[&source]).unwrap();
        assert!(
            unit.to_ir_text().contains(".shared-constant $ EARLY null"),
            "{expression}"
        );
        common::pipeline(unit);
    }
    for expression in [
        "[1,2]",
        "[-1]",
        "[+1]",
        "[+(1)]",
        "[(1)]",
        "[+(-1)]",
        "[1,2]b",
        "[\"x\"]",
        "[-2147483649l]",
        "[-2.5d]",
    ] {
        let source = format!("const EARLY=VALUES;const VALUES={expression};function f(){{}}");
        let unit = compile_sources(&[&source]).unwrap();
        assert!(
            !unit.to_ir_text().contains(".shared-constant $ EARLY null"),
            "{expression}"
        );
        common::pipeline(unit);
    }
}
