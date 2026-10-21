use ciq_compiler::{
    DictionaryKey, DictionaryValue, Operation, ParameterType, ReturnType,
    api::{ApiContext, ApiLevel, Type},
    compile, compile_sources, compile_with_api, parse_ir, parse_ir_with_api, parse_unit_ir,
};

#[test]
fn byte_number_calls_keep_numeric_results_options_and_saved_contracts() {
    for source in [
        "function f(b as Lang.ByteArray) as Lang.Numeric {return b.decodeNumber(Lang.NUMBER_FORMAT_UINT32,{});}",
        "function f(b as Lang.ByteArray,format as Lang.NumberFormat) as Lang.Numeric {var opts={:offset=>1,:endianness=>Lang.ENDIAN_BIG};return b.decodeNumber(format,opts);}",
        "function f(b as Lang.ByteArray,value as Lang.Numeric) as Lang.ByteArray {return b.encodeNumber(value,Lang.NUMBER_FORMAT_FLOAT,{:endianness=>Lang.ENDIAN_LITTLE});}",
        "function f(n as Lang.Numeric) as Void {System.println(n);n.toString();n.toNumber();n.toLong();n.toFloat();n.toDouble();n.format(\"%f\");}",
        "function f(n as Lang.Numeric) as Toybox.Lang.Number {if(n instanceof Lang.Number){return n+1;}return 0;}",
        "function f(n as Lang.Numeric) as Lang.Numeric {var local=n;local=1;local=2l;local=3.0;local=4.0d;return local;}",
        "function f(o as Lang.Object) as Lang.Numeric {return o as Lang.Numeric;}",
        "using Toybox.Lang as L;function f(b as L.ByteArray) as L.ByteArray {return b.encodeNumber(2.5d,L.NUMBER_FORMAT_FLOAT,{:offset=>0});}",
    ] {
        let f = compile(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        let asm = f.lower();
        assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    }
    let f=compile("function f(b as Lang.ByteArray) as Lang.Numeric {return b.decodeNumber(Lang.NUMBER_FORMAT_UINT32,{});}").unwrap();
    assert_eq!(f.return_type(), ReturnType::Numeric);
    assert!(parse_ir(&f.to_ir_text().replace("result=numeric", "result=integer")).is_err());
    let f = compile(
        "function f(n as Lang.Numeric) as Toybox.Lang.Number {if(n instanceof Lang.Number){return n+1;}return 0;}",
    )
    .unwrap();
    assert_eq!(f.parameter_types(), &[ParameterType::Numeric]);
    assert!(
        parse_ir(
            &f.to_ir_text()
                .replace("Toybox.Lang.Number", "Toybox.Lang.Float")
        )
        .is_err()
    );
    let unit=compile_sources(&["module B {function decode(b as Lang.ByteArray) as Lang.Numeric {return b.decodeNumber(Lang.NUMBER_FORMAT_FLOAT,{});}function forward(b as Lang.ByteArray) as Lang.Numeric {return B.decode(b);}} "]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn byte_numeric_and_option_operands_are_checked_without_emitting_conversions() {
    for source in [
        "function f(n as Lang.Numeric or Null) as Lang.Numeric {return n+1;}",
        "function f(b as Lang.ByteArray) as Toybox.Lang.Number {return b.decodeNumber(Lang.NUMBER_FORMAT_UINT32,{});}",
        "function f(b as Lang.ByteArray) as Void {b.encodeNumber(true,Lang.NUMBER_FORMAT_UINT8,{});}",
        "function f(b as Lang.ByteArray) as Void {b.encodeNumber(null,Lang.NUMBER_FORMAT_UINT8,{});}",
        "function f(b as Lang.ByteArray) as Void {b.decodeNumber(null,{});}",
        "function f(b as Lang.ByteArray) as Void {b.decodeNumber(Lang.NUMBER_FORMAT_FLOAT,null);}",
        "function f(b as Lang.ByteArray) as Void {b.decodeNumber(Lang.NUMBER_FORMAT_FLOAT,{\"offset\"=>0});}",
        "function f(b as Lang.ByteArray) as Void {b.decodeNumber(Lang.NUMBER_FORMAT_FLOAT,{:offset=>1.0});}",
        "function f(b as Lang.ByteArray) as Void {b.decodeNumber(Lang.NUMBER_FORMAT_FLOAT,{:offset=>null});}",
        "function f(b as Lang.ByteArray) as Void {b.decodeNumber(Lang.NUMBER_FORMAT_FLOAT);}",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    // Explicit enum assertions and invalid numeric ranges remain runtime checks.
    for source in [
        "function f(b as Lang.ByteArray) as Void {b.decodeNumber(99 as Lang.NumberFormat,{});}",
        "function f(b as Lang.ByteArray) as Void {b.encodeNumber(256,Lang.NUMBER_FORMAT_UINT8,{:offset=>-1});}",
    ] {
        assert!(compile(source).is_ok(), "{source}");
    }
    let api = ciq_compiler::api::core();
    let signature = api.method("Toybox.Lang.ByteArray", "decodeNumber").unwrap();
    assert!(matches!(&signature.method.parameters[1],Type::Options(fields) if fields.len()==2));
    assert_eq!(*signature.result, Type::numeric());
}

#[test]
fn numeric_constants_and_calls_recheck_api_levels_even_when_unused() {
    let current = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(3, 1, 0)).unwrap();
    let old = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(3, 0, 0)).unwrap();
    for source in [
        "function f() as Lang.NumberFormat {return Lang.NUMBER_FORMAT_FLOAT;}",
        "function f() as Lang.Endian {return Lang.ENDIAN_BIG;}",
        "function f(b as Lang.ByteArray,f as Lang.NumberFormat) as Void {b.decodeNumber(f,{});}",
        "function f(b as Lang.ByteArray,f as Lang.NumberFormat) as Void {b.encodeNumber(1,f,{});}",
    ] {
        let f = compile_with_api(source, current).unwrap();
        assert_eq!(f, parse_ir_with_api(&f.to_ir_text(), current).unwrap());
        assert!(compile_with_api(source, old).is_err(), "{source}");
        assert!(parse_ir_with_api(&f.to_ir_text(), old).is_err());
    }
}

#[test]
fn byte_keys_keep_identity_contracts_enumeration_and_nonnull_proofs() {
    for source in [
        "function f(k as Lang.ByteArray) as Lang.Dictionary<Lang.ByteArray,Lang.Number> {return {k=>1};}",
        "function f(d as Lang.Dictionary<Lang.ByteArray,Lang.Number>) as Lang.Array<Lang.ByteArray> {return d.keys();}",
        "function f(d as Lang.Dictionary<Lang.ByteArray,Lang.Number>,k as Lang.ByteArray) as Void {d[k]=2;d.put(k,3);d.hasKey(k);d.remove(k);}",
        "function f(k as Lang.ByteArray or Null) as Lang.Dictionary<Lang.ByteArray,Lang.Number> {if(k!=null){return {k=>1};}return {};}",
    ] {
        let f = compile(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        if source.contains("k=>1") {
            assert!(
                f.operations()
                    .iter()
                    .any(|op| matches!(op.operation, Operation::DictionaryBegin { .. }))
            );
        }
    }
    for source in [
        "function f(d as Lang.Dictionary<Lang.ByteArray,Lang.Number>,k as Lang.ByteArray or Null) as Void {d.put(k,1);}",
        "function f(d as Lang.Dictionary<Lang.ByteArray,Lang.Number>) as Void {d.put([1],1);}",
        "function f(d as Lang.Dictionary<Lang.ByteArray or Null,Lang.Number>) as Void {}",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    let f=compile("function f(k as Lang.ByteArray) as Lang.Dictionary<Lang.ByteArray,Lang.Number> {return {k=>1};}").unwrap();
    assert!(
        matches!(f.return_type(),ReturnType::TypedDictionary(ty) if ty.key==DictionaryKey::ByteArray && ty.value==DictionaryValue::Number)
    );
    let old = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(2, 4, 0)).unwrap();
    assert!(parse_ir_with_api(&f.to_ir_text(), old).is_err());
}

#[test]
fn option_records_recheck_fields_enum_identity_and_builder_completion() {
    for source in [
        "function f(b as Lang.ByteArray) as Void {b.decodeNumber(0,{});}",
        "function f(b as Lang.ByteArray) as Void {b.decodeNumber(Lang.ENDIAN_BIG,{});}",
        "function f(b as Lang.ByteArray) as Void {b.decodeNumber(Lang.NUMBER_FORMAT_FLOAT,{:endianness=>1});}",
        "function f(b as Lang.ByteArray) as Void {b.decodeNumber(Lang.NUMBER_FORMAT_FLOAT,{:offset=>Lang.ENDIAN_BIG});}",
        "function f(b as Lang.ByteArray) as Void {b.decodeNumber(Lang.NUMBER_FORMAT_FLOAT,{:unknown=>0});}",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    for source in [
        "function f(b as Lang.ByteArray,e as Lang.Endian) as Void {var local=e;var opts={:endianness=>local};b.decodeNumber(Lang.NUMBER_FORMAT_FLOAT,opts);}",
        "function f(b as Lang.ByteArray,flag as Lang.Boolean) as Void {b.decodeNumber(Lang.NUMBER_FORMAT_FLOAT,flag?{}:{:offset=>1});}",
        "function f(b as Lang.ByteArray,flag as Lang.Boolean) as Void {b.decodeNumber(Lang.NUMBER_FORMAT_FLOAT,{:offset=>flag?0:1,:endianness=>flag?Lang.ENDIAN_BIG:Lang.ENDIAN_LITTLE});}",
    ] {
        let f = compile(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    let text = "ciq-ir\n.function f\n.byte-options-begin &0 2\n.symbol %0 offset\n.integer %1 0\n.dictionary-fill &0 0 %0 %1\n.symbol %2 endianness\n.constant %3 Toybox.Lang ENDIAN_BIG\n.dictionary-fill &0 1 %2 %3\n.dictionary-finish %4 &0\n.discard %4\n.integer %5 1\n.return %5\n.end\n";
    assert_eq!(text, parse_ir(text).unwrap().to_ir_text());
    for edited in [
        text.replace("begin &0 2", "begin &0 3"),
        text.replace("begin &0 2", "begin &0 1"),
        text.replace("fill &0 1", "fill &0 0"),
        text.replace("%0 offset", "%0 unknown"),
        text.replace("ENDIAN_BIG", "NUMBER_FORMAT_FLOAT"),
        text.replace(".integer %1 0", ".constant %1 Toybox.Lang ENDIAN_LITTLE"),
    ] {
        assert!(parse_ir(&edited).is_err(), "accepted {edited}");
    }
}
