use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    compile, compile_sources, compile_sources_with_api, parse_ir, parse_unit_ir,
};

#[test]
fn scalar_assertions_resume_without_runtime_checks_or_conversions() {
    for (class, tag) in [
        ("Number", "integer"),
        ("Long", "long"),
        ("Float", "float"),
        ("Double", "double"),
        ("Boolean", "boolean"),
        ("Char", "char"),
        ("String", "string"),
        ("Object", "object"),
        ("Method", "method"),
        ("Symbol", "symbol"),
        ("WeakReference", "weak-reference"),
        ("Dictionary", "dictionary"),
    ] {
        let source = format!(
            "function f(o as Toybox.Lang.Object) as Toybox.Lang.{class} {{ return o as Toybox.Lang.{class}; }}"
        );
        let function = compile(&source).unwrap();
        let text = function.to_ir_text();
        assert!(text.contains(&format!(" {tag} span=")), "{text}");
        assert_eq!(function, parse_ir(&text).unwrap());
        assert!(!function.lower().to_text().contains("isa"));
    }
    let uncast = compile("function f() { return 3; }").unwrap();
    let cast = compile("function f() { return 3 as Lang.String as Lang.Number; }").unwrap();
    assert_eq!(uncast.lower(), cast.lower());
    let source = "function f(o as Toybox.Lang.Object) { return 1 + o as Lang.Number; }";
    let function = compile(source).unwrap();
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    assert!(compile("function f(o as Toybox.Lang.Object) { return -o as Lang.Number; }").is_err());
    assert!(compile("function f(o as Toybox.Lang.Object) { return -(o as Lang.Number); }").is_ok());
}

#[test]
fn alias_nullable_logical_and_conditional_casts_preserve_type_contracts() {
    for source in [
        "using Toybox.Lang as L; function f(o as Toybox.Lang.Object) as Toybox.Lang.String { return o as L.String; }",
        "function f(o as Toybox.Lang.Object) { return (o as Lang.String).length(); }",
        "function f(o as Toybox.Lang.Object or Null) as Toybox.Lang.String or Null { return o as Lang.String or Null; }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Boolean { return (o as Lang.Boolean) or false; }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Boolean { return o as Lang.Boolean or false; }",
        "function f(flag as Toybox.Lang.Boolean) { return (flag ? 1 : 2) as Lang.Number; }",
        "function f(o as Toybox.Lang.Object or Null) as Toybox.Lang.String { return o as Lang.String; }",
    ] {
        let function = compile(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
        function.lower();
    }
    let source = "module M { function length(s as Toybox.Lang.String) { return s.length(); } function f(o as Toybox.Lang.Object) { return M.length(o as Lang.String); } }";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn unsupported_targets_and_newer_api_types_are_rejected() {
    for target in [
        "Missing",
        "Lang.Void",
        "Null",
        "Lang.Array",
        "Lang.Comparator",
        "Lang.String or Lang.Number",
    ] {
        let source =
            format!("function f(o as Toybox.Lang.Object) {{ var value = o as {target}; }}");
        assert!(compile(&source).is_err(), "{source}");
    }
    for tag in [
        "void",
        "null",
        "array-number",
        "source(C@Application.AppBase)",
        "unknown",
    ] {
        let text =
            format!("ciq-ir\n.function f\n.integer %0 1\n.cast %1 %0 {tag}\n.return %1\n.end\n");
        assert!(parse_ir(&text).is_err());
    }
    let source =
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Char { return o as Lang.Char; }";
    let low = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap();
    assert!(compile_sources_with_api(&[source], low).is_err());
    let high = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 3, 0)).unwrap();
    let unit = compile_sources_with_api(&[source], high).unwrap();
    assert!(unit.validate_api(low).is_err());
}

#[test]
fn cast_chains_preserve_the_supported_nesting_limit() {
    let casts = " as Lang.Number".repeat(64);
    compile(&format!("function f() {{ return 1{casts}; }}")).unwrap();
    let casts = " as Lang.Number".repeat(65);
    assert!(compile(&format!("function f() {{ return 1{casts}; }}")).is_err());
}
