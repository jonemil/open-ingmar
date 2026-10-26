#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{
    DictionaryType, DictionaryValue, Operation, ParameterType,
    api::{ApiContext, ApiLevel},
    compile_function, compile_function_with_api, compile_sources, parse_function_ir,
    parse_function_ir_with_api, parse_unit_ir,
};

#[test]
fn byte_dictionary_values_preserve_construction_inference_and_typed_enumeration() {
    for source in [
        "function f() as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray> {return {};}",
        "function f() as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray> {var d={\"a\"=>[7]b};return d;}",
        "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray> {return {\"a\"=>b?[1]b:[2]b};}",
        "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray or Null> {return {\"empty\"=>null,\"live\"=>b?[1]b:null};}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.Number,Toybox.Lang.ByteArray>) as Toybox.Lang.Array<Toybox.Lang.ByteArray> {return d.values();}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.Symbol,Toybox.Lang.ByteArray or Null>) as Toybox.Lang.Array<Toybox.Lang.ByteArray or Null> {return d.values();}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray>) {var b=d[\"a\"];if(b!=null){d.remove(\"a\");return b[0];}return 0;}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray or Null>) as Void {d.put(\"a\",[1]b);d[\"a\"]=null;var b=d.get(\"a\");if(b!=null){b.add(2);}}",
    ] {
        let f = compile_function(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        let asm = f.lower();
        assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    }
    let source = "module B {function make() as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray> {return {\"a\"=>[1]b};}function use() as Void {var d=B.make();d.put(\"b\",[2]b);}}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let f=compile_function("function f(b as Toybox.Lang.ByteArray) as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray> {return {\"a\"=>b};}").unwrap();
    assert!(
        f.operations()
            .iter()
            .any(|op| matches!(op.operation, Operation::DictionaryBegin { .. }))
    );
    assert!(
        !f.operations()
            .iter()
            .any(|op| matches!(op.operation, Operation::DictionaryConstant { .. }))
    );
}

#[test]
fn dictionary_values_cannot_erase_class_nullability_or_lookup_proofs() {
    for source in [
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray>) {return d[\"a\"].size();}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray>) {if(d.hasKey(\"a\")){return d.get(\"a\")[0];}return 0;}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray>) {if(d[\"a\"]!=null){return d[\"a\"].size();}return 0;}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray>) as Void {d.put(\"a\",null);}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray>) as Void {d[\"a\"]=[1];}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray>) as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray or Null> {return d;}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray or Null>) as Toybox.Lang.Array<Toybox.Lang.ByteArray> {return d.values();}",
    ] {
        assert!(
            checking::compile_function(source).is_err(),
            "accepted {source}"
        );
    }
    let f=compile_function("function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray>) as Void {var b=d[\"a\"];if(b!=null){b.size();}}").unwrap();
    assert!(parse_function_ir(&f.to_ir_text().replace(".not-equal ", ".equal ")).is_err());
}

#[test]
fn dictionary_null_constants_and_unused_types_keep_payload_and_api_checks() {
    for value in [
        DictionaryValue::ByteArray,
        DictionaryValue::NullableByteArray,
    ] {
        let ty = DictionaryType {
            key: ciq_compiler::DictionaryKey::String,
            value: value.clone(),
        };
        let suffix = if value == DictionaryValue::NullableByteArray {
            " or Null"
        } else {
            ""
        };
        let source = format!(
            "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray{suffix}>) as Void {{}}"
        );
        assert_eq!(
            compile_function(&source).unwrap().parameter_types(),
            &[ParameterType::TypedDictionary(ty)]
        );
    }
    let modern = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(3, 0, 0)).unwrap();
    let old = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(2, 4, 0)).unwrap();
    for source in [
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray>) as Void {}",
        "function f() as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.ByteArray or Null> {return {\"a\"=>null};}",
    ] {
        let f = compile_function_with_api(source, modern).unwrap();
        let text = f.to_ir_text();
        assert_eq!(f, parse_function_ir(&text).unwrap());
        assert!(compile_function_with_api(source, old).is_err());
        assert!(parse_function_ir_with_api(&text, old).is_err());
        if source.contains("=>null") {
            assert!(
                f.operations()
                    .iter()
                    .any(|op| matches!(op.operation, Operation::DictionaryConstant { .. }))
            );
            let edited = text.replace(" null span=", " number 0x00000001 span=");
            assert_ne!(text, edited);
            assert!(parse_function_ir(&edited).is_err());
        }
    }
}
