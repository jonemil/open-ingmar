use ciq_assembly::Program;
use ciq_compiler::{
    api::{self, ApiContext, ApiLevel, Type},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources_with_api, compile_with_api, parse_ir, parse_unit_ir,
};

#[test]
fn module_values_preserve_complete_saved_stages_and_explicit_bindings() {
    let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(3, 1, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[include_str!(
            "../../../examples/compiler-lang-format/app.mc"
        )],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [99; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
    let assembly = unit.lower();
    assert_eq!(assembly, Program::parse(&assembly.to_text()).unwrap());
    assert!(
        assembly
            .to_text()
            .contains("getmv @__ciq_sdk_Lang @__ciq_native_format\nfrpush")
    );

    let f = compile(
        "function f(a as Lang.Array<Lang.Number>) as Lang.String {return Lang.format(\"$1$\", a);}",
    )
    .unwrap();
    let symbols = Program::parse(
        ".symbol lang \"Toybox_Lang\" id=0x800007\n.symbol format \"format\" id=0x80009c",
    )
    .unwrap();
    let bindings = ciq_compiler::symbol_bindings(&f.lower(), &symbols).unwrap();
    assert_eq!(bindings["__ciq_sdk_Lang"], 0x800007);
    assert_eq!(bindings["__ciq_native_format"], 0x80009c);
    let assembly = f.lower();
    let strings = assembly
        .items()
        .iter()
        .filter_map(|item| match item {
            ciq_assembly::Item::String(record) => Some((record.alias.clone(), 0)),
            _ => None,
        })
        .collect();
    assembly
        .encode_with_references(&bindings, &strings)
        .unwrap();
}

#[test]
fn format_accepts_erased_arrays_without_mutable_covariance() {
    let context = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let declaration = context
        .function(api::core(), "Toybox.Lang", "format")
        .unwrap();
    assert_eq!(
        declaration.parameters,
        vec![
            Type::named("Toybox.Lang.String"),
            Type::named("Toybox.Lang.Array")
        ]
    );
    assert_eq!(declaration.result, Type::named("Toybox.Lang.String"));
    for source in [
        "using Toybox.Lang as Text; function f() as Text.String {return Text.format(\"$1$\", [1,\"two\",null]);}",
        "function f() as Lang.String {return Toybox.Lang.format(\"plain\", []);}",
        "function f(a as Lang.Array<Lang.Number>) as Lang.String {return Lang.format(\"$1$\", a);}",
        "function f(a as Lang.Array<Lang.Numeric>) as Lang.String {return Lang.format(\"$1$\", a);}",
        "function f(a as Lang.Array<Lang.Object or Null>) as Lang.String {return Lang.format(\"$1$\", a);}",
        "function f() as Lang.String {var s = Lang.format(\"$1$\", [true,1l,2.5,3.5d]);return s.toUpper();}",
        "function f() as Void {Lang.format(\"$1$\", [1]);}",
    ] {
        let f = compile_with_api(source, context).unwrap_or_else(|e| panic!("{source}: {e:?}"));
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    assert!(
        compile("function f(a as Lang.Array<Lang.Number>) as Lang.Array<Lang.Object> {return a;}")
            .is_err()
    );
}

#[test]
fn native_module_calls_recheck_contracts_and_saved_ir_edits() {
    for body in [
        "return Lang.format();",
        "return Lang.format(\"x\");",
        "return Lang.format(\"x\", [], 1);",
        "return Lang.format(null, []);",
        "return Lang.format(1, []);",
        "return Lang.format(\"x\", null);",
        "return Lang.format(\"x\", [1]b);",
        "return Lang.format(\"x\", {});",
        "return Lang.format(\"x\", 1);",
        "return Lang.missing(\"x\", []);",
        "return System.exit();",
        "return WatchUi.requestUpdate();",
    ] {
        assert!(
            compile(&format!("function f() as Lang.String {{{body}}}")).is_err(),
            "{body}"
        );
    }
    assert!(
        compile("function f(a as Lang.Object) as Lang.String {return Lang.format(\"$1$\", a);}")
            .is_err()
    );
    assert!(compile("function f() {return Lang.format(\"x\", []);}").is_err());
    let text = compile("function f() as Lang.String {return Lang.format(\"$1$\", [1]);}")
        .unwrap()
        .to_ir_text();
    for edited in [
        text.replace("Toybox.Lang format", "Lang format"),
        text.replace("Toybox.Lang format", "Toybox.System format"),
        text.replace("Toybox.Lang format", "Toybox.Lang missing"),
        text.replace(".call-native-module ", ".call-native-module-discard "),
        text.replace("Toybox.Lang format", "Toybox.Lang format %999"),
    ] {
        assert!(parse_ir(&edited).is_err(), "{edited}");
    }
    for invalid in [
        ".call-native-module",
        ".call-native-module %0",
        ".call-native-module %0 Toybox.Lang",
    ] {
        let edited = text
            .lines()
            .map(|line| {
                if line.starts_with(".call-native-module ") {
                    invalid
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(parse_ir(&edited).is_err());
    }
    let text = compile(
        "function f(a as Lang.Array<Lang.Number>) as Lang.String {return Lang.format(\"$1$\", a);}",
    )
    .unwrap()
    .to_ir_text();
    let mut lines = text.lines().map(str::to_owned).collect::<Vec<_>>();
    let string = lines
        .iter()
        .position(|line| line.starts_with(".string "))
        .unwrap();
    let array = lines
        .iter()
        .position(|line| line.starts_with(".parameter "))
        .unwrap();
    lines.swap(string, array);
    let error = parse_ir(&lines.join("\n")).unwrap_err();
    assert!(error.message.contains("evaluation order"), "{error}");
}

#[test]
fn formatting_callbacks_cannot_bypass_required_field_initialization() {
    for call in ["Lang.format(\"$1$\", [1]);", "Helpers.format();"] {
        let source = format!(
            r#"
            module Helpers {{ function format() as Void {{Lang.format("$1$", [1]);}} }}
            class Target {{ function initialize() {{}} }}
            class C {{
                var value as Target;
                function initialize(t as Target) {{{call} self.value = t;}}
            }}
        "#
        );
        let error = ciq_compiler::compile_sources(&[&source]).unwrap_err();
        assert!(error.diagnostic.message.contains("shutdown"), "{error:?}");
    }
}
