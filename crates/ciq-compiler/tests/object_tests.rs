use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, compile_sources_with_api, parse_ir, parse_unit_ir,
};

const SOURCE_TYPES: &str = include_str!("../../../examples/compiler-object-tests/source-types.mc");
const APP: &str = include_str!("../../../examples/compiler-object-tests/app.mc");

#[test]
fn predicates_survive_saved_ir_assembly_and_complete_app_paths() {
    let unit = compile_sources(&[APP, SOURCE_TYPES]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".instanceof"));
    assert!(text.contains("Toybox.Lang.Number"));
    assert!(text.contains(".has"));
    let resumed = parse_unit_ir(&text).unwrap();
    assert_eq!(unit, resumed);
    let asm = unit.lower();
    assert!(asm.to_text().contains("isa"));
    assert!(asm.to_text().contains("hasv"));
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    let options = ConsoleApplication {
        id: [0x52; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
}

#[test]
fn qualified_names_aliases_and_api_requirements_remain_checked() {
    for class in ["Toybox.Lang.Number", "Lang.Number", "L.Number"] {
        let source = format!(
            "using Toybox.Lang as L; function f(o as Toybox.Lang.Object) as Toybox.Lang.Boolean {{ return o instanceof {class}; }}"
        );
        let function = compile(&source).unwrap();
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    }
    let source = "function f(o as Toybox.Lang.Object) as Toybox.Lang.Boolean { return o instanceof Lang.Char; }";
    let low = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap();
    assert!(compile_sources_with_api(&[source], low).is_err());
    let accepted = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 3, 0)).unwrap();
    let unit = compile_sources_with_api(&[source], accepted).unwrap();
    assert!(unit.validate_api(low).is_err());
    for source in [
        "function f(o as Toybox.Lang.Object) { return o instanceof Lang.Comparator; }",
        "function f(o as Toybox.Lang.Object) { return o instanceof Missing; }",
        "function f(o as Toybox.Lang.Object) { return o has 1; }",
        "function f(o as Toybox.Lang.Object or Null) { return o has :toString; }",
        "function f() { return null has :toString; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn saved_ir_cannot_claim_unknown_runtime_classes_or_non_symbol_selectors() {
    let unit = compile_sources(&[APP, SOURCE_TYPES]).unwrap();
    let text = unit.to_ir_text();
    for changed in [
        text.replace("Toybox.Lang.Number", "Toybox.Lang.Comparator"),
        text.replace("Toybox.Lang.Number", "Missing"),
    ] {
        assert_ne!(text, changed);
        assert!(parse_unit_ir(&changed).is_err());
    }
    assert!(parse_ir("ciq-ir\n.function f result=boolean\n.integer %0 1\n.integer %1 2\n.has %2 %0 %1\n.return %2\n.end\n").is_err());
    assert!(parse_ir("ciq-ir\n.function f result=boolean\n.null %0\n.symbol %1 toString\n.has %2 %0 %1\n.return %2\n.end\n").is_err());
}
