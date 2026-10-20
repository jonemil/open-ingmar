use ciq_compiler::{
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, parse_unit_ir,
};

fn prg(source: &str) -> Vec<u8> {
    let unit = compile_sources(&[source]).unwrap();
    let options = ConsoleApplication {
        id: [3; 16],
        entry: "Counter.start".into(),
    };
    let native = build_console_application(&unit, &options).unwrap();
    let resumed =
        build_console_application(&parse_unit_ir(&unit.to_ir_text()).unwrap(), &options).unwrap();
    assert_eq!(native, resumed);
    native.to_bytes()
}

#[test]
fn native_spellings_resolve_to_the_same_complete_application() {
    let canonical = prg(
        "class Counter extends Application.AppBase { function start() { System.println(41); } }",
    );
    for source in [
        "using Toybox.Application as App; using Toybox.System as Sys; class Counter extends App.AppBase { function start() { Sys.println(41); } }",
        "import Toybox.Application; import Toybox.System; class Counter extends AppBase { function start() { System.println(41); } }",
        "class Counter extends Toybox.Application.AppBase { function start() { Toybox.System.println(41); } }",
        "class Counter extends Application.AppBase { using Toybox.System as Sys; function start() { Sys.println(41); } }",
    ] {
        assert_eq!(prg(source), canonical, "{source}");
    }
}

#[test]
fn single_function_and_nested_print_arguments_use_native_bindings() {
    let source = "using Toybox.System as Sys; function main() { Sys.println(Numbers.answer()); }";
    let unit = compile_sources(&[
        source,
        "module Numbers { function answer() { return 41; } }",
    ])
    .unwrap();
    assert!(unit.to_ir_text().contains(".println"));
    assert!(compile("using Toybox.System as Sys; function main() { Sys.println(41); }").is_ok());
    assert!(compile("function main() { Toybox.System.println(41); }").is_ok());
}

#[test]
fn lexical_bindings_do_not_leak_between_owners_or_files() {
    let owner = "module A { using Toybox.System as Sys; function first() { Sys.println(1); } }";
    let own_module = "module Sys { function println(n) {} }";
    let caller = "function main() { Sys.println(41); }";
    let unit = compile_sources(&[owner, own_module, caller]).unwrap();
    assert!(unit.to_ir_text().contains(".call-discard Sys println"));
    let unit = compile_sources(&[&format!("{owner} {own_module} {caller}")]).unwrap();
    assert!(unit.to_ir_text().contains(".call-discard Sys println"));
    let error = compile_sources(&[owner, caller]).unwrap_err();
    assert_eq!(error.source, 1);
    assert!(error.diagnostic.message.contains("Sys.println"));
    assert!(compile_sources(&[
        "module A { import Toybox.Application; function f() {} } class C extends AppBase { function f() {} }"
    ]).is_err());
}

#[test]
fn using_does_not_import_bare_native_classes() {
    assert!(
        compile_sources(&["using Toybox.Application; class C extends AppBase { function f() {} }"])
            .is_err()
    );
    assert!(
        compile_sources(&[
            "using Toybox.Application; class C extends Application.AppBase { function f() {} }"
        ])
        .is_ok()
    );
}

#[test]
fn unsupported_bindings_and_shadowing_have_source_diagnostics() {
    for (source, expected) in [
        (
            "using Toybox.Sensor; function f() {}",
            "unsupported native module",
        ),
        (
            "import Toybox.System as Sys; function f() {}",
            "import aliases",
        ),
        (
            "using Toybox.System as Application; function f() {}",
            "conflicting native binding",
        ),
        (
            "using Toybox.System as Sys; using Toybox.Application as Sys; function f() {}",
            "conflicting native binding",
        ),
        (
            "using Toybox.System as Sys; function f(Sys) {}",
            "shadows native binding",
        ),
        (
            "using Toybox.System as Sys; function f() { var Sys = 1; }",
            "shadows native binding",
        ),
        (
            "using Toybox.System as Sys; module Sys { function f() {} }",
            "shadows native binding",
        ),
        (
            "using Toybox.System as Sys; function f() { Sys.println(); }",
            "one argument",
        ),
        (
            "function f() { Toybox.Sensor.foo(); }",
            "unsupported qualified module path",
        ),
    ] {
        let error = compile_sources(&["function first() {}", source]).unwrap_err();
        assert_eq!(error.source, 1);
        assert!(
            error.diagnostic.message.contains(expected),
            "{source}: {error}"
        );
    }
}

#[test]
fn source_keywords_are_reserved_even_before_their_construct_is_implemented() {
    for word in [
        "alias",
        "and",
        "as",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "default",
        "do",
        "else",
        "enum",
        "extends",
        "false",
        "finally",
        "for",
        "function",
        "has",
        "hidden",
        "if",
        "import",
        "instanceof",
        "me",
        "module",
        "NaN",
        "native",
        "new",
        "null",
        "or",
        "private",
        "protected",
        "public",
        "return",
        "self",
        "static",
        "switch",
        "throw",
        "true",
        "try",
        "typedef",
        "using",
        "var",
        "while",
    ] {
        for source in [
            format!("function {word}() {{ return 1; }}"),
            format!("function probe({word}) {{ return 1; }}"),
            format!("function probe() {{ var {word} = 1; return 1; }}"),
            format!("module {word} {{ function probe() {{ return 1; }} }}"),
            format!("class {word} {{ function probe() {{ return 1; }} }}"),
            format!("class Counter {{ var {word} = 1; function probe() {{ return 1; }} }}"),
            format!("class Counter {{ function {word}() {{ return 1; }} }}"),
            format!("using Toybox.Lang as {word}; function probe() {{ return 1; }}"),
        ] {
            let error = compile_sources(&[&source]).unwrap_err();
            assert!(
                error.to_string().contains("non-reserved identifier"),
                "{source}: {error}"
            );
        }
    }
}

#[test]
fn source_keywords_do_not_restrict_saved_ir_or_assembly_names() {
    let function =
        ciq_compiler::parse_ir("ciq-ir\n.function for\n.integer %0 41\n.return %0\n.end\n")
            .unwrap();
    assert_eq!(function.name(), "for");
    assert!(ciq_assembly::Program::parse("for:\nipush1 0x29\nreturn\n").is_ok());
}
