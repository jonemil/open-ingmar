use ciq_compiler::{
    ParameterType,
    application::{ConsoleApplication, build_console_application},
    compile_sources, parse_unit_ir,
};
use ciq_prg::data::classes::{Record, records};

fn example() -> ciq_compiler::CompilationUnit {
    compile_sources(&[
        include_str!("../../../examples/compiler-lifecycle/app.mc"),
        include_str!("../../../examples/compiler-lifecycle/helpers.mc"),
    ])
    .unwrap()
}
fn options(entry: &str) -> ConsoleApplication {
    ConsoleApplication {
        id: [5; 16],
        entry: entry.into(),
    }
}

#[test]
fn source_startup_preserves_its_native_signature_and_complete_prg() {
    let unit = example();
    let method = unit.classes()[0]
        .methods
        .iter()
        .find(|f| f.name() == "onStart")
        .unwrap();
    assert_eq!(
        method.parameter_types(),
        &[ParameterType::NullableDictionary]
    );
    let text = unit.to_ir_text();
    assert!(text.contains("parameter-types=nullable-dictionary"));
    assert!(text.contains(".null"));
    let native = build_console_application(&unit, &options("StartupApp.onStart")).unwrap();
    let resumed = build_console_application(
        &parse_unit_ir(&text).unwrap(),
        &options("StartupApp.onStart"),
    )
    .unwrap();
    assert_eq!(native, resumed);
    let data = records(native.unique_section(ciq_prg::DATA).unwrap().payload()).unwrap();
    let starts = data
        .iter()
        .filter_map(|r| {
            if let Record::Class(c) = r {
                Some(&c.fields)
            } else {
                None
            }
        })
        .flatten()
        .filter(|f| f.symbol_id() == 0x0080029f)
        .count();
    assert_eq!(starts, 1);
    assert!(method.assembly().contains("npush"));
    assert!(method.assembly().contains("lgetv 0x01"));
    assert!(build_console_application(&unit, &options("StartupApp.printAnswer")).is_err());
}

#[test]
fn callback_state_cannot_be_used_as_an_integer_or_string() {
    for body in [
        "System.println(state);",
        "var x = state + 1;",
        "return state;",
        "if (state > 0) {}",
        "if (state == 0) {}",
        "self.take(state);",
        "var x = state; x = 1;",
        "var x = 1; x = state;",
        "return 1;",
        "self.onStart(1);",
    ] {
        let source = format!(
            "class C extends Application.AppBase {{
            function onStart(state) {{ {body} }} function take(n) {{}} }}"
        );
        assert!(compile_sources(&[&source]).is_err(), "{body}");
    }
    for declaration in ["function onStart() {}", "function onStart(a,b) {}"] {
        let source = format!("class C extends Application.AppBase {{ {declaration} }}");
        assert!(compile_sources(&[&source]).is_err());
    }
    assert!(
        compile_sources(&["class C extends Application.AppBase {
        function onStart(state) { var copy = state; copy = null;
            if (null == copy) { System.println(1); } } }"])
        .is_ok()
    );
}

#[test]
fn saved_ir_retains_and_checks_callback_types() {
    let text = example().to_ir_text();
    for bad in [
        text.replace(" parameter-types=nullable-dictionary", ""),
        text.replace(
            "parameter-types=nullable-dictionary",
            "parameter-types=integer",
        ),
        text.replace(
            "parameter-types=nullable-dictionary",
            "parameter-types=unknown",
        ),
        text.replace(
            "parameter-types=nullable-dictionary",
            "parameter-types=integer,nullable-dictionary",
        ),
    ] {
        assert!(parse_unit_ir(&bad).is_err());
    }
}
