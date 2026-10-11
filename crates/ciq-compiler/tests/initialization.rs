use ciq_compiler::{
    application::{ConsoleApplication, build_console_application},
    compile_sources, parse_unit_ir,
};
use ciq_prg::data::classes::{Record, records};

fn app() -> ConsoleApplication {
    ConsoleApplication {
        id: [4; 16],
        entry: "C.start".into(),
    }
}

#[test]
fn native_base_spellings_and_saved_ir_preserve_the_complete_application() {
    let mut expected = None;
    for call in [
        "AppBase.initialize()",
        "Application.AppBase.initialize()",
        "App.AppBase.initialize()",
        "Toybox.Application.AppBase.initialize()",
    ] {
        let source = format!(
            "using Toybox.Application as App; class C extends App.AppBase {{
            function initialize() {{ {call}; System.println(1); }}
            function start() {{ System.println(41); }} }}"
        );
        let unit = compile_sources(&[&source]).unwrap();
        let direct = build_console_application(&unit, &app()).unwrap();
        let text = unit.to_ir_text();
        assert!(text.contains(".base-initialize Application.AppBase"));
        let resumed = build_console_application(&parse_unit_ir(&text).unwrap(), &app()).unwrap();
        assert_eq!(direct, resumed);
        if let Some(bytes) = &expected {
            assert_eq!(&direct.to_bytes(), bytes);
        } else {
            expected = Some(direct.to_bytes());
        }
        let data = records(direct.unique_section(ciq_prg::DATA).unwrap().payload()).unwrap();
        let fields = data.iter().filter_map(|r| {
            if let Record::Class(c) = r {
                Some(&c.fields)
            } else {
                None
            }
        });
        assert_eq!(
            fields
                .flatten()
                .filter(|f| f.symbol_id() == 0x0080005e)
                .count(),
            1
        );
        let asm = unit.lower().to_text();
        assert!(asm.contains("getselfv @__ciq_native_AppBase"));
        assert!(asm.contains("getsv @__ciq_native_initialize"));
        let mut options = app();
        options.entry = "C.initialize".into();
        assert!(build_console_application(&unit, &options).is_err());
    }
}

#[test]
fn initializer_contract_rejects_unsafe_or_unsupported_source_forms() {
    for method in [
        "function initialize() {}",
        "function initialize(n) { AppBase.initialize(); }",
        "function initialize() { AppBase.initialize(); return 1; }",
        "function initialize() { System.println(1); AppBase.initialize(); }",
        "function initialize() { AppBase.initialize(); AppBase.initialize(); }",
        "function initialize() { if (1 == 1) { AppBase.initialize(); } }",
        "function initialize() { AppBase.initialize(1); }",
        "function initialize() { var n = AppBase.initialize(); }",
        "function initialize() { AppBase.other(); }",
        "function other() { AppBase.initialize(); }",
        "function initialize(AppBase) { AppBase.initialize(); }",
    ] {
        let source =
            format!("class C extends Application.AppBase {{ {method} function start() {{}} }}");
        assert!(compile_sources(&[&source]).is_err(), "{method}");
    }
}

#[test]
fn saved_ir_cannot_bypass_initializer_contracts() {
    let unit = compile_sources(&["class C extends Application.AppBase {
        function initialize() { AppBase.initialize(); } function start() {} }"])
    .unwrap();
    let text = unit.to_ir_text();
    for bad in [
        text.replace(
            ".base-initialize Application.AppBase",
            ".base-initialize Other",
        ),
        text.replace(
            ".base-initialize Application.AppBase",
            ".base-initialize Application.AppBase\n.base-initialize Application.AppBase",
        ),
        text.replace(".base-initialize Application.AppBase", ""),
    ] {
        assert!(parse_unit_ir(&bad).is_err());
    }
}
