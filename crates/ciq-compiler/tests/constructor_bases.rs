use ciq_compiler::{ClassBase, Operation, compile_sources, parse_function_ir, parse_unit_ir};

const APP: &str = include_str!("../../../examples/compiler-visible-view/app.mc");
const VIEW: &str = include_str!("../../../examples/compiler-visible-view/view.mc");

#[test]
fn forward_source_construction_carries_the_declared_native_base() {
    let unit = compile_sources(&[APP, VIEW]).unwrap();
    let initial = unit.classes()[0]
        .methods
        .iter()
        .find(|method| method.name() == "getInitialView")
        .unwrap();
    assert!(initial.operations().iter().any(|located| matches!(
        &located.operation,
        Operation::SourceNew { class, base: ClassBase::WatchUiView, .. }
            if class == "HelloView"
    )));
    let text = unit.to_ir_text();
    assert!(text.contains("HelloView WatchUi.View"));
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
}

#[test]
fn unit_rejects_a_constructor_projection_that_disagrees_with_its_class() {
    let text = compile_sources(&[APP, VIEW]).unwrap().to_ir_text();
    // Keep the native callback contract; only the unit knows the changed
    // constructor's source class actually extends AppBase rather than View.
    let changed = text
        .lines()
        .map(|line| {
            if line.starts_with(".new-source ") || line.starts_with(".begin-source ") {
                line.replace("HelloView WatchUi.View", "MinimalConsoleApp WatchUi.View")
            } else if line.starts_with(".function getInitialView ") {
                line.replace("tuple(source(HelloView@WatchUi.View))", "tuple(view)")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let error = parse_unit_ir(&changed).unwrap_err();
    assert!(error.to_string().contains("native base"), "{error}");
}

#[test]
fn standalone_constructor_requires_an_explicit_supported_projection() {
    let prefix = "ciq-ir\n.function f\n";
    let suffix = "\n.return %0\n.end\n";
    let valid = format!(
        "{prefix}.begin-source &0 HelloView WatchUi.View\n.new-source %0 HelloView WatchUi.View &0{suffix}"
    );
    let function = parse_function_ir(&valid).unwrap();
    assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
    for constructor in [
        ".new-source %0 HelloView",
        ".new-source %0 HelloView Application.AppBase",
        ".new-source %0 HelloView Graphics.Dc",
        ".new-source %0 HelloView Unknown",
    ] {
        assert!(parse_function_ir(&format!("{prefix}{constructor}{suffix}")).is_err());
    }
}

#[test]
fn a_module_with_methods_is_not_a_constructible_source_class() {
    let source = "module Helper { function initialize() {} }\n\
        function f() { return new Helper(); }";
    let error = compile_sources(&[source]).unwrap_err();
    assert!(error.to_string().contains("declared class"), "{error}");
}
