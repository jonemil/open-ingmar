use ciq_assembly::{Item, Program};
use ciq_compiler::{
    ArrayElement, DictionaryKey, DictionaryType, DictionaryValue, ReturnType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, compile_sources_with_api, compile_with_api, parse_ir,
    parse_ir_with_api, parse_unit_ir,
};

#[test]
fn object_containers_preserve_complete_saved_stages() {
    let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(3, 1, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[include_str!(
            "../../../examples/compiler-object-containers/app.mc"
        )],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [98; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
    assert_eq!(
        unit.lower(),
        Program::parse(&unit.lower().to_text()).unwrap()
    );
}

#[test]
fn object_constants_keep_scalar_classes_bits_order_and_nullability() {
    for nullable in [false, true] {
        let suffix = if nullable { " or Null" } else { "" };
        let null = if nullable { ",null" } else { "" };
        let source = format!(
            "function f() as Lang.Array<Lang.Object{suffix}> {{return [1,1l,-0.0,1.0d,'A',true,\"text\"{null}];}}"
        );
        let f = compile(&source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        assert_eq!(
            f.return_type(),
            ReturnType::Array(if nullable {
                ArrayElement::NullableObject
            } else {
                ArrayElement::LangObject
            })
        );
        let asm = f.lower();
        let mut record = asm
            .items()
            .iter()
            .find_map(|item| {
                if let Item::Array(a) = item {
                    Some(a.clone())
                } else {
                    None
                }
            })
            .unwrap();
        let data = ciq_prg::data::Source::parse(&record.to_text()).unwrap();
        assert!(ciq_compiler::array_bindings(&asm, &data).is_ok());
        record.values.swap(0, 1);
        let data = ciq_prg::data::Source::parse(&record.to_text()).unwrap();
        assert!(ciq_compiler::array_bindings(&asm, &data).is_err());
        let source = format!(
            "function f() as Lang.Dictionary<Lang.String,Lang.Object{suffix}> {{return {{\"n\"=>1,\"l\"=>1l,\"f\"=>-0.0,\"d\"=>1.0d,\"c\"=>'A',\"b\"=>true,\"s\"=>\"text\"{}}};}}",
            if nullable { ",\"null\"=>null" } else { "" }
        );
        let f = compile(&source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        assert_eq!(
            f.return_type(),
            ReturnType::TypedDictionary(DictionaryType {
                key: DictionaryKey::String,
                value: if nullable {
                    DictionaryValue::NullableObject
                } else {
                    DictionaryValue::LangObject
                }
            })
        );
        let text = f.to_ir_text();
        if nullable {
            assert!(
                parse_ir(&text.replace(
                    "dictionary(string=>nullable-object)",
                    "dictionary(string=>object)"
                ))
                .is_err()
            );
        }
    }
    let source = "function f() as Lang.Array<Lang.Object or Null> {return [null];}";
    let f = compile(source).unwrap();
    assert!(
        parse_ir(&f.to_ir_text().replace(
            ".array-constant %0 nullable-object",
            ".array-constant %0 object"
        ))
        .is_err()
    );
}

#[test]
fn object_storage_preserves_context_erasure_and_nullable_snapshot_proofs() {
    for source in [
        "function f(a as Lang.Array<Lang.Object>) {return a.size();}",
        "function f() as Lang.Array<Lang.Object> {return [];}",
        "function f() as Lang.Array<Lang.Object or Null> {return [];}",
        "function f() as Lang.Dictionary<Lang.String,Lang.Object> {return {};}",
        "function f(o as Lang.Object) as Lang.Array<Lang.Object> {var a=[o,1];return a;}",
        "function f(o as Lang.Object) as Lang.Dictionary<Lang.String,Lang.Object> {var d={\"o\"=>o};return d;}",
        "function f(o as Lang.Object or Null) as Lang.Dictionary<Lang.String,Lang.Object or Null> {var d={\"o\"=>o};return d;}",
        "function f(a as Lang.Array<Lang.Object>) as Void {a.addAll([1,\"text\",[2],{1=>2},[3]b,:symbol]);}",
        "function f(a as Lang.Array<Lang.Object or Null>) as Lang.String {var o=a[0];if(o!=null){a[0]=null;return o.toString();}return \"null\";}",
        "function f(d as Lang.Dictionary<Lang.String,Lang.Object>) as Lang.String {var o=d[\"key\"];if(o!=null){d.remove(\"key\");return o.toString();}return \"null\";}",
        "function f(d as Lang.Dictionary<Lang.String,Lang.Object>) as Lang.Array<Lang.Object> {return d.values();}",
        "function f(d as Lang.Dictionary<Lang.String,Lang.Object or Null>) as Lang.Array<Lang.Object or Null> {return d.values();}",
        "function f(n) as Lang.Array<Lang.Object or Null> {return new[n];}",
        "function f(a as Lang.Array<Lang.Object>,flag as Lang.Boolean) as Lang.Array<Lang.Object> {return flag?a:a;}",
    ] {
        let f = compile(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        let text = f.to_ir_text();
        if text.contains(".not-equal ") {
            assert!(parse_ir(&text.replace(".not-equal ", ".equal ")).is_err());
        }
    }
    let unit=compile_sources(&["class C {function initialize(){}function get(){return 1;}}module M {function f(a as Lang.Array<Lang.Object>){var o=a[0];if(o instanceof C){return o.get();}return 0;}} "]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let source = "class C {function initialize(){}}function f() as Lang.Array<Lang.Object>{var a=[new C()];return a;}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn object_storage_does_not_invent_mutable_covariance_or_recover_erased_types() {
    for source in [
        "function f(a as Lang.Array<Lang.Number>) as Lang.Array<Lang.Object> {return a;}",
        "function f(a as Lang.Array<Lang.Object>) as Lang.Array<Lang.Object or Null> {return a;}",
        "function f(d as Lang.Dictionary<Lang.String,Lang.Number>) as Lang.Dictionary<Lang.String,Lang.Object> {return d;}",
        "function f(a as Lang.Array<Lang.Object>) as Void {a.add(null);}",
        "function f(d as Lang.Dictionary<Lang.String,Lang.Object>) as Void {d.put(\"n\",null);}",
        "function f(a as Lang.Array<Lang.Object>) {return a[0].size();}",
        "function f(a as Lang.Array<Lang.Object>) {return a[0]+1;}",
        "function f(a as Lang.Array<Lang.Object or Null>) as Lang.String {return a[0].toString();}",
        "function f(a as Lang.Array<Lang.Object or Null>) as Lang.String {if(a[0]!=null){return a[0].toString();}return \"null\";}",
        "function f(d as Lang.Dictionary<Lang.String,Lang.Object>) as Lang.String {return d[\"n\"].toString();}",
        "function f(d as Lang.Dictionary<Lang.String,Lang.Object>) as Void {d.values().add(null);}",
        "function f() as Lang.Array<Lang.Object> {return new[2];}",
        "function f() as Lang.Dictionary<Lang.Object,Lang.Object> {return {};}",
        "function f(a as Lang.Array<Lang.Object>) as Lang.Array<Lang.Object> {return a as Lang.Array<Lang.Object>;}",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn object_erasure_retains_api_requirements_and_enum_nominality() {
    let old = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap();
    let char_api = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 3, 0)).unwrap();
    for source in [
        "function f() as Lang.Array<Lang.Object> {return ['A'];}",
        "function f() as Lang.Dictionary<Lang.String,Lang.Object> {return {\"c\"=>'A'};}",
    ] {
        let f = compile_with_api(source, char_api).unwrap();
        assert!(compile_with_api(source, old).is_err());
        assert!(parse_ir_with_api(&f.to_ir_text(), old).is_err());
    }
    for value in [
        "Lang.NUMBER_FORMAT_FLOAT",
        "Lang.ENDIAN_BIG",
        "Toybox.Graphics.FONT_MEDIUM",
    ] {
        let source = format!("function f() as Lang.Object {{return {value};}}");
        let f = compile(&source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        let source = format!(
            "module M {{function id(o as Lang.Object) as Lang.Object{{return o;}}function f() as Lang.Object{{return M.id({value});}}}}"
        );
        let unit = compile_sources(&[&source]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
        assert!(compile(&format!("function f(){{return {value}+1;}}")).is_err());
        assert!(compile(&format!("function f() as Lang.Number{{return {value};}}")).is_err());
    }
}
