use ciq_compiler::{compile_sources, parse_function_ir, parse_unit_ir};
mod common;

const BIND: &str = "ciq-ir\n.function bind parameters=2 parameter-types=any:symbol\n.method-begin &0\n.parameter %0 0\n.parameter %1 1\n.method-finish %2 &0 %0 %1\n.return %2\n.end\n";

#[test]
fn construction_keeps_all_saved_stages_and_complete_application() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-method-construction/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".method-begin"));
    assert!(text.contains(".method-finish"));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0xb67c1691c9a5422a9d3202a0c016870cu128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    for source in [
        "function f(){return new Toybox.Lang.Method(-1,:abs);}",
        "using Toybox.Lang as L;function f(){return new L.Method(\"abc\",:length);}",
        "import Toybox.Lang;function f(){return new Method(1,:abs);}",
        "function f(){return new $.Toybox.Lang.Method(1,:abs);}",
        "module M{function number(){return 1;}function bind(){return new Toybox.Lang.Method(me,:number);}}",
        "function number(){return 1;}function bind(){return new Toybox.Lang.Method(self,:number);}",
        "class C{function value(){return 1;}function bind(){return new Toybox.Lang.Method(self,:value);}}",
        "function f(flag as Toybox.Lang.Boolean){return new Toybox.Lang.Method(flag?[1]:[2],flag ? :size : :size);}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}")));
    }
}

#[test]
fn arity_selectors_qualifiers_and_receiver_readiness_are_checked() {
    for source in [
        "function f(){return new Toybox.Lang.Method();}",
        "function f(){return new Toybox.Lang.Method(1);}",
        "function f(){return new Toybox.Lang.Method(1,:abs,1);}",
        "function f(){return new Toybox.Lang.Method(1,1);}",
        "function f(){return new Toybox.Lang.Method(1,\"abs\");}",
        "function f(){return new Toybox.Lang.Method(1,null);}",
        "using Toybox.Lang as L;function f(L as Toybox.Lang.Number){return new L.Method(1,:abs);}",
        "class C{var n as Toybox.Lang.Number;function initialize(){var m=new Toybox.Lang.Method(self,:value);n=1;}function value(){return n;}}",
        "function f(m as Toybox.Lang.Method){m.initialize(1,:abs);}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let unit =
        compile_sources(&["function f(m as Toybox.Lang.Method){return m.method(:initialize);}"])
            .unwrap();
    assert!(!unit.to_ir_text().contains(".bind-method"));
}

#[test]
fn private_method_builders_require_matching_completion_on_every_path() {
    let function = parse_function_ir(BIND).unwrap();
    assert_eq!(function.return_type(), ciq_compiler::ReturnType::Method);
    assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
    for bad in [
        BIND.replace(".method-begin &0\n", ""),
        BIND.replace(".method-begin &0", ".method-begin &0\n.method-begin &0"),
        BIND.replace(".method-finish %2 &0", ".method-finish %2 &99"),
        BIND.replace("parameter-types=any:symbol", "parameter-types=any:integer"),
        BIND.replace(".method-begin &0", ".array-begin &0 number 1"),
        BIND.replace(".method-finish %2 &0 %0 %1", ".null %2"),
        BIND.replace(".return %2", ".method-finish %3 &0 %0 %1\n.return %3"),
        BIND.replace(".method-finish %2 &0 %0 %1", ".return %0"),
    ] {
        assert!(parse_function_ir(&bad).is_err(), "{bad}");
    }
    let unit = compile_sources(&[
        "function f(flag as Toybox.Lang.Boolean){return new Toybox.Lang.Method(flag?1:2,:abs);}",
    ])
    .unwrap();
    let text = unit.to_ir_text();
    common::pipeline(parse_unit_ir(&text).unwrap());
}

#[test]
fn any_selectors_follow_checking_policy_and_constructor_api() {
    let source = "function f(value,selector){return new Toybox.Lang.Method(value,selector);}";
    let options = ciq_compiler::CompileOptions {
        type_check: ciq_compiler::TypeCheckLevel::Informative,
        ..Default::default()
    };
    let output = ciq_compiler::compile_sources_with_options(&[source], options).unwrap();
    assert!(
        output
            .warnings
            .iter()
            .any(|d| d.kind == ciq_compiler::TypeDiagnosticKind::AmbiguousBoundary)
    );
    let strict = ciq_compiler::CompileOptions {
        type_check: ciq_compiler::TypeCheckLevel::Strict,
        ..Default::default()
    };
    assert!(ciq_compiler::compile_sources_with_options(&[source], strict).is_err());
    let initializer = ciq_compiler::api::core()
        .method("Toybox.Lang.Method", "initialize")
        .unwrap();
    assert_eq!(
        initializer.required_api,
        ciq_compiler::api::ApiLevel(1, 0, 0)
    );
    assert_eq!(initializer.method.parameters.len(), 2);
}

#[test]
fn constructor_nesting_keeps_the_existing_depth_budget() {
    let nested = |depth: usize| {
        let mut value = "1".to_owned();
        for _ in 0..depth {
            value = format!("new Toybox.Lang.Method({value},:toString)");
        }
        format!("function f(){{return {value};}}")
    };
    compile_sources(&[&nested(64)]).unwrap();
    assert!(compile_sources(&[&nested(65)]).is_err());
}
