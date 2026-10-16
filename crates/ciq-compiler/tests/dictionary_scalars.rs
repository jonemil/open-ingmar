use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    compile, compile_sources, compile_with_api, parse_ir, parse_ir_with_api, parse_unit_ir,
};

#[test]
fn scalar_dictionary_values_preserve_constants_computed_entries_and_saved_stages() {
    for (ty, literal, other) in [("Boolean", "false", "'A'"), ("Char", "'A'", "false")] {
        for value in [literal.to_owned(), format!("{literal}, \"absent\" => null")] {
            let source =
                format!("function f() {{ var d = {{\"one\" => {value}}}; return d.size(); }}");
            let f = compile(&source).unwrap();
            assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
            let assembly = f.lower();
            assert_eq!(
                assembly,
                ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
            );
        }
        for literal in [
            "{}".to_owned(),
            "{\"one\" => null}".to_owned(),
            format!("{{\"one\" => {literal}}}"),
        ] {
            let source = format!(
                "function f() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.{ty} or Null> {{ return {literal}; }}"
            );
            let f = compile(&source).unwrap();
            assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
            assert_eq!(
                compile(&source.replace(" or Null", "")).is_ok(),
                !literal.contains("null")
            );
        }
        let source = format!(
            "module D {{ function scalar(value as Toybox.Lang.{ty} or Null) as Toybox.Lang.{ty} or Null {{ return value; }} function f(p as Toybox.Lang.Boolean) {{ var d = {{\"first\" => p ? D.scalar({literal}) : null, \"second\" => null}}; d.put(\"third\", {literal}); return d.size(); }} }}"
        );
        let unit = compile_sources(&[&source]).unwrap();
        let text = unit.to_ir_text();
        assert_eq!(unit, parse_unit_ir(&text).unwrap());
        assert!(compile_sources(&[&source.replace("=> null", &format!("=> {other}"))]).is_err());
        for mixed in [
            format!("{{\"same\" => {literal}, \"same\" => 1}}"),
            format!("{{\"one\" => {literal}, \"two\" => {other}}}"),
        ] {
            assert!(compile(&format!("function f() {{ return {mixed}.size(); }}")).is_err());
        }
    }
}

#[test]
fn nullable_lookup_requires_storage_proofs_and_mutable_contracts_are_invariant() {
    for (ty, literal, other, consume, tag) in [
        ("Boolean", "false", "'A'", "!value", "boolean"),
        ("Char", "'A'", "false", "value + 1", "char"),
    ] {
        for suffix in ["", " or Null"] {
            let receiver =
                format!("Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.{ty}{suffix}>");
            let source = format!(
                "function f(d as {receiver}) as Toybox.Lang.{ty} {{ var value = d.get(\"one\"); if (value == null) {{ return {literal}; }} d.remove(\"one\"); return {consume}; }}"
            );
            let f = compile(&source).unwrap();
            let text = f.to_ir_text();
            assert_eq!(f, parse_ir(&text).unwrap());
            assert!(parse_ir(&text.replace(".equal", ".not-equal")).is_err());
            for body in [
                "System.println(d[\"one\"]);".to_owned(),
                "if (d.hasKey(\"one\")) { System.println(d[\"one\"]); }".to_owned(),
                format!("d.put(\"one\", {other});"),
                format!("d[\"one\"] = {other};"),
                "d.put(null, null);".to_owned(),
            ] {
                assert!(compile(&format!("function f(d as {receiver}) {{ {body} }}")).is_err());
            }
        }
        let source = format!(
            "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.{ty}>) {{ d.put(\"one\", null); }}"
        );
        assert!(compile(&source).is_err());
        let source = format!(
            "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.{ty} or Null>) as Toybox.Lang.Array<Toybox.Lang.{ty}> {{ return d.values(); }}"
        );
        assert!(compile(&source).is_err());
        for (parameter, result) in [
            (ty.to_owned(), format!("{ty} or Null")),
            (format!("{ty} or Null"), ty.to_owned()),
        ] {
            let source = format!(
                "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.{parameter}>) as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.{result}> {{ return d; }}"
            );
            assert!(compile(&source).is_err());
        }
        let source = format!(
            "module D {{ function make() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.{ty} or Null> {{ return {{\"one\" => {literal}}}; }} function f() {{ return D.make().size(); }} }}"
        );
        let unit = compile_sources(&[&source]).unwrap();
        let text = unit.to_ir_text();
        let edited = text.replace(
            &format!("make result=dictionary(string=>nullable-{tag})"),
            &format!("make result=dictionary(string=>{tag})"),
        );
        assert_ne!(text, edited);
        assert!(parse_unit_ir(&edited).is_err());
    }
}

#[test]
fn char_values_require_char_api_even_with_non_char_keys() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let char_api = ApiContext::new(ApiLevel(1, 3, 0), ApiLevel(1, 3, 0)).unwrap();
    for (ty, accepted) in [("Boolean", true), ("Char", false)] {
        for suffix in ["", " or Null"] {
            let source = format!(
                "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.{ty}{suffix}>) {{ return d.size(); }}"
            );
            let f = compile(&source).unwrap();
            assert_eq!(compile_with_api(&source, initial).is_ok(), accepted);
            assert_eq!(
                parse_ir_with_api(&f.to_ir_text(), initial).is_ok(),
                accepted
            );
            assert!(compile_with_api(&source, char_api).is_ok());
        }
    }
}
