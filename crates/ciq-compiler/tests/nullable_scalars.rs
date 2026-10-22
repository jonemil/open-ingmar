use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    compile_function, compile_function_with_api, compile_sources, parse_function_ir, parse_unit_ir,
};

#[test]
fn boolean_and_char_null_guards_preserve_saved_stages() {
    for (ty, literal, consume) in [("Boolean", "false", "!value"), ("Char", "'A'", "value + 1")] {
        for guard in ["value != null", "null != value", "!(value == null)"] {
            let source = format!(
                "function f(value as Toybox.Lang.{ty} or Null) as Toybox.Lang.{ty} \
                 {{ if ({guard}) {{ return {consume}; }} return {literal}; }}"
            );
            let f = compile_function(&source).unwrap();
            assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
            let assembly = f.lower();
            assert_eq!(
                assembly,
                ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
            );
            let text = f.to_ir_text();
            let edited = text.replace(".not-equal", ".equal");
            if edited != text {
                assert!(parse_function_ir(&edited).is_err());
            }
        }
        for source in [
            format!("function f() as Toybox.Lang.{ty} or Null {{ return null; }}"),
            format!("function f() as Toybox.Lang.{ty} or Null {{ return {literal}; }}"),
            format!(
                "function f(p as Toybox.Lang.Boolean) as Toybox.Lang.{ty} or Null {{ return p ? {literal} : null; }}"
            ),
        ] {
            let f = compile_function(&source).unwrap();
            assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        }
    }
}

#[test]
fn nullable_scalars_require_proofs_and_preserve_storage_identity() {
    for (ty, literal, consume) in [("Boolean", "false", "!value"), ("Char", "'A'", "value + 1")] {
        for body in [
            format!("return {consume};"),
            format!("if (value == null) {{ return {consume}; }} return {literal};"),
            format!(
                "var copy = value; if (copy != null) {{ return {consume}; }} return {literal};"
            ),
            format!(
                "var tested = value != null; if (tested) {{ return {consume}; }} return {literal};"
            ),
            format!("if (value != null) {{ return {literal}; }} return {consume};"),
            format!("System.println(value); return {literal};"),
        ] {
            let source = format!(
                "function f(value as Toybox.Lang.{ty} or Null) as Toybox.Lang.{ty} {{ {body} }}"
            );
            assert!(compile_function(&source).is_err(), "accepted {source}");
        }
        let source = format!(
            "function f(p as Toybox.Lang.Boolean) as Toybox.Lang.{ty} {{ var value = p ? {literal} : null; if (value == null) {{ return {literal}; }} value = null; return {consume}; }}"
        );
        assert!(compile_function(&source).is_err());
        let source = format!(
            "function f(value as Toybox.Lang.{ty} or Null) as Toybox.Lang.{ty} or Null {{ return 42; }}"
        );
        assert!(compile_function(&source).is_err());
    }
}

#[test]
fn unit_calls_cannot_erase_or_swap_nullable_results() {
    for (ty, literal, tag, other) in [
        ("Boolean", "false", "boolean", "char"),
        ("Char", "'A'", "char", "boolean"),
    ] {
        let source = format!(
            "module Values {{ function pass(value as Toybox.Lang.{ty} or Null) as Toybox.Lang.{ty} or Null {{ return value; }} }} function f() as Toybox.Lang.{ty} or Null {{ return Values.pass({literal}); }}"
        );
        let unit = compile_sources(&[&source]).unwrap();
        let text = unit.to_ir_text();
        assert_eq!(unit, parse_unit_ir(&text).unwrap());
        for replacement in [tag.to_owned(), format!("nullable-{other}")] {
            let edited = text.replace(
                &format!("pass result=nullable-{tag}"),
                &format!("pass result={replacement}"),
            );
            assert_ne!(text, edited);
            assert!(parse_unit_ir(&edited).is_err());
        }
        assert!(compile_sources(&[&source.replace(literal, "42")]).is_err());
    }
}

#[test]
fn nullable_char_retains_the_char_api_minimum() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    for (ty, accepted) in [("Boolean", true), ("Char", false)] {
        let source = format!(
            "function f(value as Toybox.Lang.{ty} or Null) as Toybox.Lang.{ty} or Null {{ return value; }}"
        );
        assert_eq!(
            compile_function_with_api(&source, initial).is_ok(),
            accepted
        );
    }
}
