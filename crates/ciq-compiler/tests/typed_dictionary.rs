use ciq_compiler::{
    DictionaryKey, DictionaryType, DictionaryValue, Operation, ReturnType, compile,
    compile_sources, parse_ir, parse_unit_ir,
};

const NUMBER: &str = "Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number>";

#[test]
fn empty_construction_retains_typed_contracts_without_changing_vm_allocation() {
    for (key, key_kind) in [
        ("Number", DictionaryKey::Number),
        ("String", DictionaryKey::String),
        ("Boolean", DictionaryKey::Boolean),
        ("Char", DictionaryKey::Char),
    ] {
        for (value, value_kind) in [
            ("Number", DictionaryValue::Number),
            ("String", DictionaryValue::String),
            ("Number or Null", DictionaryValue::NullableNumber),
            ("String or Null", DictionaryValue::NullableString),
        ] {
            let ty = DictionaryType {
                key: key_kind,
                value: value_kind,
            };
            let source = format!(
                "function f() as Toybox.Lang.Dictionary<Toybox.Lang.{key}, Toybox.Lang.{value}> {{ return {{}}; }}"
            );
            let f = compile(&source).unwrap();
            assert_eq!(f.return_type(), ReturnType::TypedDictionary(ty));
            assert!(
                matches!(f.operations()[0].operation, Operation::DictionaryNew { dictionary_type: Some(actual), .. } if actual == ty)
            );
            assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
            assert_eq!(f.lower().encode().unwrap(), [0x35, 1, 0x37, 0x2f, 0x16]);
            assert_eq!(
                f.lower(),
                ciq_assembly::Program::parse(&f.lower().to_text()).unwrap()
            );
        }
    }
}

#[test]
fn parameters_calls_and_local_reassignment_keep_invariant_types() {
    let source = format!(
        "module D {{
        function make() as {NUMBER} {{ return {{}}; }}
        function copy(d as {NUMBER}) as {NUMBER} {{ return d; }}
        function erase(d as Toybox.Lang.Dictionary) as Toybox.Lang.Dictionary {{ return d; }}
        function count(d as {NUMBER}) {{ var local = d; local = {{}}; return local.size(); }}
        function run() {{ var d = D.copy(D.make()); d = {{}};
            return D.count({{}}) + D.erase(d).size() + d.size(); }}
    }}"
    );
    let unit = compile_sources(&[&source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert_eq!(
        unit.lower(),
        ciq_assembly::Program::parse(&unit.lower().to_text()).unwrap()
    );
    for other in [
        "Toybox.Lang.Dictionary<Toybox.Lang.Number, Toybox.Lang.Number>",
        "Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.String>",
        "Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number or Null>",
        "Toybox.Lang.Dictionary",
    ] {
        let source = format!(
            "module D {{ function take(d as {NUMBER}) {{ return d.size(); }}
            function pass(d as {other}) {{ return D.take(d); }} }}"
        );
        assert!(compile_sources(&[&source]).is_err(), "accepted {source}");
    }
    let opaque = format!("function f(d as Toybox.Lang.Dictionary) as {NUMBER} {{ return d; }}");
    assert!(compile(&opaque).is_err());
}

#[test]
fn saved_ir_does_not_recover_erased_or_incompatible_allocation_types() {
    let source = format!("function f() as {NUMBER} {{ return {{}}; }}");
    let text = compile(&source).unwrap().to_ir_text();
    assert!(text.contains("result=dictionary(string=>integer)"));
    assert!(text.contains(".dictionary-new %0 type=dictionary(string=>integer)"));
    assert!(parse_ir(&text.replace(" type=dictionary(string=>integer)", "")).is_err());
    assert!(
        parse_ir(&text.replace(
            "type=dictionary(string=>integer)",
            "type=dictionary(string=>string)"
        ))
        .is_err()
    );
    for bad in [
        "dictionary(string)",
        "dictionary(string=>boolean)",
        "dictionary(null=>integer)",
        "dictionary(string=>integer=>integer)",
        "dictionary(string=>array-integer)",
    ] {
        assert!(parse_ir(&text.replace("dictionary(string=>integer)", bad)).is_err());
    }
    let source = format!(
        "module D {{ function make() as {NUMBER} {{ return {{}}; }}
        function size() {{ return D.make().size(); }} }}"
    );
    let text = compile_sources(&[&source]).unwrap().to_ir_text();
    let changed = text.replace(
        "make result=dictionary(string=>integer)",
        "make result=dictionary(integer=>integer)",
    );
    assert_ne!(text, changed);
    assert!(parse_unit_ir(&changed).is_err());
}

#[test]
fn unsupported_annotations_and_methods_remain_rejected() {
    for bad in [
        "Toybox.Lang.Object, Toybox.Lang.Number",
        "Null, Toybox.Lang.Number",
        "Toybox.Lang.String, Toybox.Lang.Boolean",
        "Toybox.Lang.String, Toybox.Lang.Char",
        "Toybox.Lang.String, Toybox.Lang.Array<Toybox.Lang.Number>",
        "Toybox.Lang.String, Toybox.Lang.Number or Toybox.Lang.String",
    ] {
        let source =
            format!("function f(d as Toybox.Lang.Dictionary<{bad}>) {{ return d.size(); }}");
        assert!(compile(&source).is_err(), "accepted {source}");
    }
    for body in [
        "return d.get(\"one\");",
        "d.put(\"one\", 1); return 0;",
        "return d[\"one\"];",
        "return true ? d : d;",
        "return {\"one\" => 1}.size();",
    ] {
        let source = format!("function f(d as {NUMBER}) {{ {body} }}");
        assert!(compile(&source).is_err(), "accepted {source}");
    }
}

#[test]
fn dictionary_key_contracts_check_api_levels_in_native_and_saved_ir() {
    use ciq_compiler::api::{ApiContext, ApiLevel};
    let source = "function f() as Toybox.Lang.Dictionary<Toybox.Lang.Char, Toybox.Lang.Number> { return {}; }";
    let early = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let later = ApiContext::new(ApiLevel(1, 3, 0), ApiLevel(1, 3, 0)).unwrap();
    assert!(ciq_compiler::compile_with_api(source, early).is_err());
    let f = ciq_compiler::compile_with_api(source, later).unwrap();
    assert!(ciq_compiler::parse_ir_with_api(&f.to_ir_text(), early).is_err());
    assert_eq!(
        f,
        ciq_compiler::parse_ir_with_api(&f.to_ir_text(), later).unwrap()
    );
}
