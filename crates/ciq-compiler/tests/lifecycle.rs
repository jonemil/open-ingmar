#[path = "common/checking.rs"]
mod checking;

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
        "var x = state; x = 1; x.size();",
        "var x = 1; x = state; x.toNumber();",
        "return 1;",
        "self.onStart(1);",
    ] {
        let source = format!(
            "class C extends Application.AppBase {{
            function onStart(state) {{ {body} }} function take(n) {{}} }}"
        );
        assert!(checking::compile_sources(&[&source]).is_err(), "{body}");
    }
    for declaration in ["function onStart() {}", "function onStart(a,b) {}"] {
        let source = format!("class C extends Application.AppBase {{ {declaration} }}");
        assert!(checking::compile_sources(&[&source]).is_err());
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

#[test]
fn shutdown_callback_uses_native_contract_and_has_one_data_binding() {
    let unit = example();
    let stop = unit.classes()[0]
        .methods
        .iter()
        .find(|f| f.name() == "onStop")
        .unwrap();
    assert_eq!(stop.parameter_types(), [ParameterType::NullableDictionary]);
    assert_eq!(stop.return_type(), ciq_compiler::ReturnType::Void);
    let native = build_console_application(&unit, &options("StartupApp.onStart")).unwrap();
    let resumed = build_console_application(
        &parse_unit_ir(&unit.to_ir_text()).unwrap(),
        &options("StartupApp.onStart"),
    )
    .unwrap();
    assert_eq!(native, resumed);
    let data = native.unique_section(ciq_prg::DATA).unwrap().payload();
    let stops = records(data)
        .unwrap()
        .iter()
        .filter_map(|r| match r {
            Record::Class(class) => Some(class.fields.iter()),
            _ => None,
        })
        .flatten()
        .filter(|f| f.symbol_id() == 0x008002a1)
        .count();
    assert_eq!(stops, 1);
    for declaration in [
        "function onStop() {}",
        "function onStop(a,b) {}",
        "function onStop(state) { return 1; }",
        "function onStop(state) { System.println(state); }",
        "function onStop(state) { var n = state + 1; }",
        "function onStop(state) { self.onStop(1); }",
    ] {
        let source = format!("class C extends Application.AppBase {{ {declaration} }}");
        assert!(compile_sources(&[&source]).is_err(), "{source}");
    }
}

#[test]
fn native_module_exit_preserves_aliases_bindings_and_effect_order() {
    for receiver in ["System", "Toybox.System", "Sys"] {
        let source = format!("using Toybox.System as Sys; function f() {{ {receiver}.exit(); }}");
        let function = ciq_compiler::compile_function(&source).unwrap();
        assert_eq!(
            function,
            ciq_compiler::parse_function_ir(&function.to_ir_text()).unwrap()
        );
        let assembly = function.lower();
        assert!(
            assembly
                .to_text()
                .contains("getmv @__ciq_sdk_System @__ciq_native_exit\ninvokemz\npopv")
        );
        let symbols = ciq_assembly::Program::parse(
            ".symbol system \"Toybox_System\" id=0x00800005\n.symbol exit \"exit\" id=0x0080006d",
        )
        .unwrap();
        let bindings = ciq_compiler::symbol_bindings(&assembly, &symbols).unwrap();
        assert_eq!(bindings["__ciq_native_exit"], 0x0080006d);
        assembly
            .encode_with_references(&bindings, &Default::default())
            .unwrap();
    }
    let source = "function f() { System.println(1); System.exit(); }";
    let function = ciq_compiler::compile_function(source).unwrap();
    let text = function.to_ir_text();
    let exit = ".call-native-module-discard Toybox.System exit";
    assert!(
        ciq_compiler::parse_function_ir(
            &text
                .replace(exit, "")
                .replace(".println %0", &format!("{exit}\n.println %0"))
        )
        .is_err()
    );
    for (source, message) in [
        ("function f() { System.exit(1); }", "argument count"),
        ("function f() { System.missing(); }", "not known"),
        ("function f() { return System.exit(); }", "value lowering"),
        ("function f(System) { System.exit(); }", "System"),
    ] {
        let error = ciq_compiler::compile_function(source).unwrap_err();
        assert!(error.message.contains(message), "{error}");
    }
    assert!(
        ciq_compiler::parse_function_ir(
            &text.replace("Toybox.System exit", "Toybox.Application exit")
        )
        .is_err()
    );
    let collision = compile_sources(&[
        "class exit extends Application.AppBase { function start() { System.exit(); } }",
    ])
    .unwrap();
    let error = build_console_application(&collision, &options("exit.start")).unwrap_err();
    assert!(error.message.contains("pinned native symbol"));
}
