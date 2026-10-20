use ciq_compiler::{
    DictionaryKey, DictionaryType, DictionaryValue, Operation, ReturnType, compile,
    compile_sources, parse_ir, parse_unit_ir,
};

#[test]
fn constants_keep_native_text_and_assembly_data_paths_equivalent() {
    let f =
        compile("function f() as Toybox.Lang.Dictionary { return {\"one\" => 1, \"two\" => 2}; }")
            .unwrap();
    assert_eq!(f.return_type(), ReturnType::Dictionary);
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let p = f.lower();
    assert_eq!(p, ciq_assembly::Program::parse(&p.to_text()).unwrap());
    let ciq_assembly::Item::Dictionary(record) = &p.items()[0] else {
        panic!("missing requirement")
    };
    let data = ciq_prg::data::Source {
        fragments: vec![ciq_prg::data::Fragment::Dictionary(record.clone())],
    };
    let bindings = ciq_compiler::dictionary_bindings(&p, &data).unwrap();
    assert_eq!(
        p.encode_with_data_references(
            &Default::default(),
            &Default::default(),
            &Default::default(),
            &bindings
        )
        .unwrap(),
        [0x35, 1, 0x42, 0, 0, 0, 0, 0x16]
    );
    assert!(p.encode().is_err());
    assert!(ciq_compiler::dictionary_bindings(&p, &ciq_prg::data::Source::default()).is_err());
    assert_eq!(data, ciq_prg::data::Source::parse(&data.to_text()).unwrap());
    let mut other = record.clone();
    other.alias = "other".into();
    let duplicate = ciq_prg::data::Source {
        fragments: vec![
            ciq_prg::data::Fragment::Dictionary(record.clone()),
            ciq_prg::data::Fragment::Dictionary(other),
        ],
    };
    // A matching identity remains usable when another record has equal contents.
    assert!(ciq_compiler::dictionary_bindings(&p, &duplicate).is_ok());
    let renamed =
        ciq_assembly::Program::parse(&p.to_text().replace(&record.alias, "requirement")).unwrap();
    assert!(ciq_compiler::dictionary_bindings(&renamed, &duplicate).is_err());
    assert!(ciq_compiler::dictionary_bindings(&renamed, &data).is_ok());
}

#[test]
fn homogeneous_inference_and_context_retain_nullable_values() {
    for (literal, key, value) in [
        (
            "{1 => \"one\", 2 => \"two\"}",
            DictionaryKey::Number,
            DictionaryValue::String,
        ),
        (
            "{'a' => 1, 'b' => null}",
            DictionaryKey::Char,
            DictionaryValue::NullableNumber,
        ),
        (
            "{true => \"yes\", false => null}",
            DictionaryKey::Boolean,
            DictionaryValue::NullableString,
        ),
        (
            "{\"é\" => -1, \"\" => -2147483648}",
            DictionaryKey::String,
            DictionaryValue::Number,
        ),
    ] {
        let source = format!("function f() {{ var d = {literal}; return d.size(); }}");
        let f = compile(&source).unwrap();
        assert!(f.operations().iter().any(|op| matches!(op.operation,
            Operation::DictionaryConstant { dictionary_type, .. } if dictionary_type == DictionaryType { key, value })));
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    let source = "module D {
        function make() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number or Null> { return {\"one\" => 1}; }
        function take(d as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number or Null>) { return d.size(); }
        function run() { var d = D.make(); d = {\"missing\" => null}; return D.take({\"null\" => null}); }
    }";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert_eq!(
        unit.lower(),
        ciq_assembly::Program::parse(&unit.lower().to_text()).unwrap()
    );
}

#[test]
fn duplicate_scalar_keys_keep_the_last_value_in_the_data_record() {
    let f = compile("function f() { return {\"same\" => 1, \"same\" => 41}.size(); }").unwrap();
    let p = f.lower();
    let ciq_assembly::Item::Dictionary(record) = &p.items()[0] else {
        panic!()
    };
    assert_eq!(record.entries.len(), 1);
    assert_eq!(
        record.entries[0].1,
        ciq_prg::data::dictionaries::Value::Number(41)
    );
    assert_eq!(record.pool.as_ref().unwrap().len(), 1);
    assert_eq!(record.pool.as_ref().unwrap()[0].value, "same");
    assert!(
        matches!(&f.operations()[0].operation, Operation::DictionaryConstant { entries, .. } if entries.len() == 2)
    );
}

#[test]
fn malformed_constants_and_unsupported_value_types_fail_before_lowering() {
    for literal in [
        "{null => 1}",
        "{\"x\" => true, \"y\" => 1}",
        "{\"x\" => 'a', \"y\" => 1}",
        "{\"x\" => null}",
        "{1 => 2, \"x\" => 3}",
        "{\"x\" => 1, \"y\" => \"bad\"}",
        "{\"x\" => {}}",
        "{\"x\" => [1]}",
        "{\"x\" => 1,,}",
        "{\"x\" = 1}",
    ] {
        assert!(
            compile(&format!("function f() {{ return {literal}.size(); }}")).is_err(),
            "accepted {literal}"
        );
    }
    let source = "function f() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number> { return {\"one\" => 1}; }";
    let text = compile(source).unwrap().to_ir_text();
    for (old, new) in [
        ("number 0x00000001", "null"),
        ("number 0x00000001", "boolean true"),
        ("number 0x00000001", "number 0x100000000"),
        ("number 0x00000001", "number"),
        (
            "dictionary(string=>integer) string",
            "dictionary(integer=>integer) string",
        ),
    ] {
        assert!(parse_ir(&text.replace(old, new)).is_err());
    }
    let opaque = "ciq-ir\n.function f\n.dictionary-constant %0 dictionary(string=>integer)\n.return %0\n.end\n";
    assert!(parse_ir(opaque).is_err());
    // Token-only profile does not make discarded overwritten entries exempt from typing.
    assert!(compile("function f() { return {\"same\" => true, \"same\" => 1}.size(); }").is_err());
}

#[test]
fn literal_limits_and_api_requirements_are_enforced_before_encoding() {
    use ciq_compiler::api::{ApiContext, ApiLevel};
    let early = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let source = "function f() { return {'a' => 1}.size(); }";
    assert!(ciq_compiler::compile_with_api(source, early).is_err());
    let f = compile(source).unwrap();
    assert!(ciq_compiler::parse_ir_with_api(&f.to_ir_text(), early).is_err());
    let long = format!(
        "function f() {{ return {{\"{}\" => 1}}.size(); }}",
        "x".repeat(65535)
    );
    assert!(compile(&long).unwrap_err().message.contains("u16"));
    let nested = format!(
        "function f() {{ return {}0{}.size(); }}",
        "{1 => ".repeat(65),
        "}".repeat(65)
    );
    assert!(compile(&nested).is_err());
    let escaped =
        compile("function f() { return {\"a #,\\\" \\\\ é\\n\" => \"same\\t\"}.size(); }").unwrap();
    assert_eq!(escaped, parse_ir(&escaped.to_ir_text()).unwrap());
    assert_eq!(
        escaped.lower(),
        ciq_assembly::Program::parse(&escaped.lower().to_text()).unwrap()
    );
}
