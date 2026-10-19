use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    compile, compile_sources, compile_with_api, parse_ir, parse_ir_with_api, parse_unit_ir,
};

#[test]
fn dictionary_identity_calls_keep_symbols_api_and_saved_stages() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    for receiver in [
        "Toybox.Lang.Dictionary",
        "Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number>",
        "Toybox.Lang.Dictionary<Toybox.Lang.Boolean, Toybox.Lang.String or Null>",
    ] {
        for (body, result, member, id) in [
            (
                "return d.equals(d);",
                "Toybox.Lang.Boolean",
                "equals",
                0x008000b3,
            ),
            (
                "return d.hashCode();",
                "Toybox.Lang.Number",
                "hashCode",
                0x008000ad,
            ),
        ] {
            let source = format!("function f(d as {receiver}) as {result} {{ {body} }}");
            let f = compile_with_api(&source, initial).unwrap();
            let text = f.to_ir_text();
            assert_eq!(f, parse_ir_with_api(&text, initial).unwrap());
            let assembly = f.lower();
            assert_eq!(
                assembly,
                ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
            );
            let symbols =
                ciq_assembly::Program::parse(&format!(".symbol native \"{member}\" id=0x{id:08x}"))
                    .unwrap();
            assert_eq!(
                ciq_compiler::symbol_bindings(&assembly, &symbols).unwrap()
                    [&format!("__ciq_native_{member}")],
                id
            );
            let edited = text.replace(&format!(" {member}"), &format!(" {member} %0"));
            assert_ne!(text, edited);
            assert!(parse_ir(&edited).is_err());
        }
    }
}

#[test]
fn dictionary_equality_accepts_bounded_cross_contract_and_nullable_arguments() {
    for argument in [
        "{}",
        "{true => false}",
        "{'a' => 'B'}",
        "null",
        "1",
        "\"one\"",
        "false",
        "'A'",
        "\"bad\".toNumber()",
        "\"x\".substring(0, 1)",
    ] {
        let source = format!(
            "function f(d as Toybox.Lang.Dictionary) as Toybox.Lang.Boolean {{ return d.equals({argument}); }}"
        );
        let f = compile(&source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    for ty in ["Number", "String", "Boolean", "Char"] {
        let source = format!(
            "function f(d as Toybox.Lang.Dictionary, value as Toybox.Lang.{ty} or Null) as Toybox.Lang.Boolean {{ return d.equals(value); }}"
        );
        let f = compile(&source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    let source = include_str!("../../../examples/compiler-dictionary/app.mc");
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    // Callback Dictionary-or-Null may be compared as an argument but cannot
    // receive either method without a non-null proof.
    for body in ["state.hashCode() == state.hashCode()", "state.equals(d)"] {
        let edited = source.replace("d.equals(state)", body);
        assert!(compile_sources(&[&edited]).is_err());
    }
}

#[test]
fn dictionary_identity_does_not_enable_unrelated_object_or_operator_profiles() {
    for body in [
        "return d.equals();",
        "return d.equals(d, d);",
        "return d == d;",
        "return d.weak(1);",
    ] {
        let source =
            format!("function f(d as Toybox.Lang.Dictionary) as Toybox.Lang.Boolean {{ {body} }}");
        assert!(compile(&source).is_err(), "accepted {source}");
    }
    for body in ["return d.hashCode(1);", "return d.equals(d) + 1;"] {
        assert!(
            compile(&format!(
                "function f(d as Toybox.Lang.Dictionary) {{ {body} }}"
            ))
            .is_err()
        );
    }
    let source =
        "function f(d as Toybox.Lang.Dictionary) as Toybox.Lang.Boolean { return d.equals(null); }";
    let text = compile(source).unwrap().to_ir_text();
    let edited = text.replace(" equals", " hashCode");
    assert_ne!(text, edited);
    assert!(parse_ir(&edited).is_err());
}
