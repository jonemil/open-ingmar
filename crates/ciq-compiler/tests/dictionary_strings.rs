use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    compile, compile_with_api, parse_ir, parse_ir_with_api,
};

#[test]
fn dictionary_strings_use_existing_native_calls_and_preserve_saved_stages() {
    for receiver in [
        "Toybox.Lang.Dictionary",
        "Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number>",
        "Toybox.Lang.Dictionary<Toybox.Lang.Number, Toybox.Lang.String or Null>",
        "Toybox.Lang.Dictionary<Toybox.Lang.Boolean, Toybox.Lang.String>",
        "Toybox.Lang.Dictionary<Toybox.Lang.Char, Toybox.Lang.Number or Null>",
    ] {
        let source =
            format!("function f(d as {receiver}) as Toybox.Lang.String {{ return d.toString(); }}");
        let f = compile(&source).unwrap();
        let text = f.to_ir_text();
        assert!(text.contains(".call-native"));
        assert!(text.contains(" toString"));
        assert_eq!(f, parse_ir(&text).unwrap());
        let p = f.lower();
        assert_eq!(p, ciq_assembly::Program::parse(&p.to_text()).unwrap());
        assert!(p.to_text().contains("__ciq_native_toString"));
        assert!(parse_ir(&text.replace(" toString", " toString %0")).is_err());
    }
    for source in [
        "function f() as Toybox.Lang.String { return {}.toString().toUpper(); }",
        "function f() as Toybox.Lang.Number { var d = {}; d.toString(); return d.toString().length(); }",
    ] {
        let f = compile(source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
}

#[test]
fn source_and_saved_ir_enforce_dictionary_override_api_level() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let introduced = ApiContext::new(ApiLevel(1, 0, 1), ApiLevel(1, 0, 1)).unwrap();
    // Object.toString predates the Dictionary override; do not fall back to it.
    for source in [
        "function f(d as Toybox.Lang.Dictionary) as Toybox.Lang.String { return d.toString(); }",
        "function f() as Toybox.Lang.String { return {\"one\" => 1}.toString(); }",
    ] {
        let f = compile_with_api(source, introduced).unwrap();
        assert_eq!(f, parse_ir_with_api(&f.to_ir_text(), introduced).unwrap());
        assert!(compile_with_api(source, initial).is_err());
        assert!(parse_ir_with_api(&f.to_ir_text(), initial).is_err());
    }
}

#[test]
fn string_conversion_requires_a_non_null_dictionary_and_correct_arity() {
    for source in [
        "function f(d as Toybox.Lang.Dictionary) as Toybox.Lang.String { return d.toString(1); }",
        "function f(d as Toybox.Lang.Dictionary) as Toybox.Lang.Number { return d.toString(); }",
        "function f() as Toybox.Lang.String { return null.toString(); }",
        "function f(d as Toybox.Lang.Dictionary) as Toybox.Lang.Number { return d.toString() + 1; }",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    let source = include_str!("../../../examples/compiler-dictionary/app.mc").replace(
        "if (state != null) { System.println(state.size()); }",
        "if (state != null) { System.println(state.toString()); }",
    );
    let unit = ciq_compiler::compile_sources(&[&source]).unwrap();
    assert_eq!(
        unit,
        ciq_compiler::parse_unit_ir(&unit.to_ir_text()).unwrap()
    );
    assert!(
        ciq_compiler::compile_sources(&[&source.replace("if (state != null)", "if (true)")])
            .is_err()
    );
}
