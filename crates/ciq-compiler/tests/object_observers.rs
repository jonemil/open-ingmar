use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, compile_with_api, parse_ir, parse_ir_with_api, parse_unit_ir,
};

#[test]
fn array_observers_preserve_element_contracts_and_inherited_api_requirements() {
    for scalar in [
        "Number", "Long", "Float", "Double", "String", "Boolean", "Char",
    ] {
        for nullable in [false, true] {
            let suffix = if nullable { " or Null" } else { "" };
            let array = format!("Toybox.Lang.Array<Toybox.Lang.{scalar}{suffix}>");
            let since = if scalar == "Char" {
                ApiLevel(1, 3, 0)
            } else {
                ApiLevel(1, 0, 0)
            };
            let api = ApiContext::new(since, since).unwrap();
            for (result, call) in [("Boolean", "equals(a)"), ("Number", "hashCode()")] {
                let source = format!(
                    "function f(a as {array}) as Toybox.Lang.{result} {{ return a.{call}; }}"
                );
                let f = compile_with_api(&source, api).unwrap();
                assert_eq!(f, parse_ir_with_api(&f.to_ir_text(), api).unwrap());
                let asm = f.lower();
                assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
            }
            let source = format!(
                "function f(a as {array}, b as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Boolean {{ return a.equals(b); }}"
            );
            let f = compile(&source).unwrap();
            assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        }
    }
    let source = "function f(a as Toybox.Lang.Array<Toybox.WatchUi.View>) as Toybox.Lang.Boolean { return a.equals(a); }";
    let f = compile(source).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let early = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let chars = source.replace("Toybox.WatchUi.View", "Toybox.Lang.Char");
    assert!(compile_with_api(&chars, early).is_err());
    assert!(parse_ir_with_api(&compile(&chars).unwrap().to_ir_text(), early).is_err());
}

#[test]
fn scalar_and_container_equality_share_the_bounded_argument_profile() {
    let arguments = [
        "[1]",
        "[1l]",
        "[1.25]",
        "[1.25d]",
        "[true]",
        "['A']",
        "[\"one\"]",
        "{}",
        "{\"one\" => 1}",
        "null",
    ];
    for receiver in [
        "1", "1l", "1.25", "1.25d", "true", "'A'", "\"one\"", "[1]", "{}",
    ] {
        for argument in arguments {
            let source = format!(
                "function f() as Toybox.Lang.Boolean {{ var a = {receiver}; return a.equals({argument}); }}"
            );
            let f = compile(&source).unwrap();
            assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        }
    }
    for ty in [
        "Number", "Long", "Float", "Double", "String", "Boolean", "Char",
    ] {
        let source = format!(
            "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>, b as Toybox.Lang.{ty} or Null) as Toybox.Lang.Boolean {{ return a.equals(b); }}"
        );
        let f = compile(&source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
}

#[test]
fn observers_do_not_enable_object_erasure_weak_callbacks_or_equality_operators() {
    for body in [
        "return a.equals();",
        "return a.equals(a,a);",
        "return a.hashCode(1);",
        "return a.equals(new Toybox.WatchUi.View());",
        "return a == a;",
        "return a.weak(1);",
        "return a.method(1);",
        "return a.sort(null);",
    ] {
        assert!(
            compile(&format!(
                "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) {{ {body} }}"
            ))
            .is_err(),
            "accepted {body}"
        );
    }
    let f = compile("function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Boolean { return a.equals(a); }").unwrap();
    let text = f.to_ir_text();
    let edited = text.replace(" equals", " hashCode");
    assert_ne!(text, edited);
    assert!(parse_ir(&edited).is_err());
    let edited = text.replace(" equals", " weak");
    assert!(parse_ir(&edited).is_err());
}

#[test]
fn observer_specimen_has_equivalent_native_saved_ir_and_assembly_paths() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-object-observers/app.mc"
    )])
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [0x25; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
}
