use ciq_assembly::Program;
use ciq_compiler::{
    ArrayElement, FieldKind, NativeClass,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_sources, compile_sources_with_api, parse_unit_ir, parse_unit_ir_with_api,
};

#[test]
fn container_fields_preserve_complete_saved_stages_and_metadata() {
    let unit = compile_sources_with_api(
        &[include_str!(
            "../../../examples/compiler-container-fields/app.mc"
        )],
        ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap(),
    )
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let assembly = unit.lower();
    assert_eq!(assembly, Program::parse(&assembly.to_text()).unwrap());
    let options = ConsoleApplication {
        id: [102; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    let prg = build_console_application(&unit, &options).unwrap();
    assert_eq!(prg, build_console_application(&resumed, &options).unwrap());
    let records =
        ciq_prg::data::classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
            .unwrap();
    let fields = records
        .iter()
        .flat_map(|record| match record {
            ciq_prg::data::classes::Record::Class(class) => class.fields.as_slice(),
            _ => &[],
        })
        .filter(|field| field.kind == 0 && field.symbol_id() < 0x0080_0000)
        .collect::<Vec<_>>();
    assert_eq!(fields.len(), 9);
    assert!(fields.iter().all(|field| field.value == 0));
    let mut classes = unit.classes().to_vec();
    classes[1].fields[0].kind = FieldKind::Array {
        element: ArrayElement::Object(NativeClass::Dc),
    };
    let error = ciq_compiler::CompilationUnit::with_classes(unit.declarations().to_vec(), classes)
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("unsupported Array field element")
    );
}

#[test]
fn field_contracts_contextualize_literals_without_container_covariance() {
    for (ty, initial, tag) in [
        ("L.Array<L.Number>", "[]", "number"),
        ("L.Array<L.Object>", "[1,\"text\",true]", "object"),
        (
            "L.Array<L.Object or Null>",
            "[null,1,\"text\"]",
            "nullable-object",
        ),
        ("L.Array<L.Number or Null>", "new[2]", "nullable-number"),
        ("L.Array<L.ByteArray>", "[]", "byte-array"),
        ("L.Array<L.Method>", "[]", "method"),
        (
            "L.Array<L.WeakReference or Null>",
            "[null]",
            "nullable-weak-reference",
        ),
        ("L.Array<L.Symbol>", "[:name]", "symbol"),
        ("L.Array<L.Numeric>", "[1,2l,3.5,4.5d]", "numeric"),
        ("L.Array<L.Comparable>", "[1,\"text\"]", "comparable"),
        (
            "L.Dictionary<L.String,L.Object>",
            "{\"n\"=>1,\"s\"=>\"text\"}",
            "dictionary(string=>object)",
        ),
        (
            "L.Dictionary<L.Integer,L.Numeric or Null>",
            "{}",
            "dictionary(integer-union=>nullable-numeric)",
        ),
    ] {
        let source = format!(
            "using Toybox.Lang as L;class C {{var value as {ty};function initialize(){{self.value={initial};}}function get() as {ty}{{return self.value;}}function set(v as {ty}) as Void{{self.value=v;}}}}"
        );
        let unit = compile_sources(&[&source]).unwrap_or_else(|e| panic!("{source}: {e:?}"));
        let text = unit.to_ir_text();
        assert!(text.contains(&format!(" {tag}\n")), "{text}");
        assert_eq!(unit, parse_unit_ir(&text).unwrap());
    }
    for (field, argument) in [
        ("Lang.Array<Lang.Object>", "Lang.Array<Lang.Number>"),
        ("Lang.Array<Lang.Number>", "Lang.Array<Lang.Object>"),
        ("Lang.Array<Lang.Object or Null>", "Lang.Array<Lang.Object>"),
        (
            "Lang.Dictionary<Lang.String,Lang.Object>",
            "Lang.Dictionary<Lang.String,Lang.Number>",
        ),
    ] {
        let source = format!(
            "class C {{var value as {field};function initialize(v as {argument}){{self.value=v;}}}}"
        );
        assert!(compile_sources(&[&source]).is_err(), "{source}");
    }
}

#[test]
fn object_fields_erase_values_but_preserve_nullability() {
    for ty in [
        "Lang.Number",
        "Lang.ByteArray",
        "Lang.Array<Lang.Number>",
        "Lang.Dictionary<Lang.String,Lang.Number>",
        "Lang.Method",
        "Lang.WeakReference",
        "Lang.Symbol",
        "Lang.Comparable",
    ] {
        let source = format!(
            "class C {{var value as Lang.Object;var maybe as Lang.Object or Null=null;function initialize(v as {ty}){{self.value=v;}}function set(v as Lang.Object or Null) as Void{{self.maybe=v;var snapshot=self.maybe;if(snapshot!=null){{self.value=snapshot;}}}}function get() as Lang.Object{{return self.value;}}}}"
        );
        let unit = compile_sources(&[&source]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
    for body in [
        "var value as Lang.Object;function initialize(){self.value=null;}",
        "var value as Lang.Object or Null;function get() as Lang.String{return self.value.toString();}",
        "var value as Lang.Object or Null;function get() as Lang.String{if(self.value!=null){return self.value.toString();}return \"\";}",
        "var value as Lang.Object;function initialize(){self.value=[1];}function get(){return self.value.size();}",
    ] {
        assert!(
            compile_sources(&[&format!("class C {{{body}}}")]).is_err(),
            "{body}"
        );
    }
}

#[test]
fn field_initialization_and_saved_edits_are_rechecked() {
    for (ty, initial, directive, tag) in [
        ("Lang.Object", "1", ".field-object", "object"),
        ("Lang.Array<Lang.Number>", "[]", ".field-array", "number"),
        (
            "Lang.Dictionary<Lang.String,Lang.Number>",
            "{}",
            ".field-dictionary",
            "dictionary(string=>integer)",
        ),
    ] {
        let source = format!(
            "class C {{var value as {ty};function initialize(){{self.value={initial};}}function get() as {ty}{{return self.value;}}}}"
        );
        let unit = compile_sources(&[&source]).unwrap();
        let text = unit.to_ir_text();
        for replacement in [
            "",
            &format!("{directive} value unknown\n"),
            &format!("{directive} value nullable-{tag}\n"),
        ] {
            let edited = text.replace(&format!("{directive} value {tag}\n"), replacement);
            assert_ne!(text, edited);
            assert!(parse_unit_ir(&edited).is_err(), "{edited}");
        }
        for setup in [
            "",
            "self.value=self.value;",
            "System.exit();",
            &format!("if(true){{self.value={initial};}}"),
            &format!("self.get();self.value={initial};"),
        ] {
            let invalid = source.replace(&format!("self.value={initial};"), setup);
            assert!(compile_sources(&[&invalid]).is_err(), "{invalid}");
        }
        for suffix in ["=null", "or Null", "=1"] {
            if ty == "Lang.Object" && suffix == "or Null" {
                continue;
            }
            let invalid = source.replace(
                &format!("var value as {ty};"),
                &format!("var value as {ty} {suffix};"),
            );
            assert!(compile_sources(&[&invalid]).is_err(), "{invalid}");
        }
    }
    assert!(
        compile_sources(&["class C {var value as Lang.Dictionary;function initialize(){}}"])
            .is_err()
    );
}

#[test]
fn field_payload_types_enforce_api_minimums_even_without_reads() {
    for (ty, initial, level) in [
        ("Lang.Object or Null", "", ApiLevel(1, 0, 0)),
        (
            "Lang.Array<Lang.Number>",
            "self.value=[];",
            ApiLevel(1, 0, 0),
        ),
        ("Lang.Array<Lang.Char>", "self.value=[];", ApiLevel(1, 3, 0)),
        (
            "Lang.Array<Lang.ByteArray>",
            "self.value=[];",
            ApiLevel(3, 0, 0),
        ),
        (
            "Lang.Array<Lang.WeakReference>",
            "self.value=[];",
            ApiLevel(1, 2, 0),
        ),
        (
            "Lang.Dictionary<Lang.String,Lang.Comparator>",
            "self.value={};",
            ApiLevel(5, 0, 0),
        ),
    ] {
        let source = format!("class C {{var value as {ty};function initialize(){{{initial}}}}}");
        let api = ApiContext::new(ApiLevel(5, 2, 0), level).unwrap();
        let unit = compile_sources_with_api(&[&source], api).unwrap();
        assert_eq!(
            unit,
            parse_unit_ir_with_api(&unit.to_ir_text(), api).unwrap()
        );
        if level > ApiLevel(1, 0, 0) {
            let old = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap();
            assert!(compile_sources_with_api(&[&source], old).is_err());
            assert!(parse_unit_ir_with_api(&unit.to_ir_text(), old).is_err());
        }
    }
}

#[test]
fn dictionary_joins_preserve_invariance_and_do_not_enable_casts() {
    let source = "function f(flag as Lang.Boolean,a as Lang.Dictionary<Lang.String,Lang.Number>,b as Lang.Dictionary<Lang.String,Lang.Number>) as Lang.Dictionary<Lang.String,Lang.Number>{return flag?a:b;}";
    let f = ciq_compiler::compile_function(source).unwrap();
    let text = f.to_ir_text();
    assert!(text.contains("dictionary(string=>integer)"));
    assert_eq!(f, ciq_compiler::parse_function_ir(&text).unwrap());
    let edited = text
        .lines()
        .map(|line| {
            if line.starts_with(".block-value ") {
                line.replace("dictionary(string=>integer)", "dictionary(string=>object)")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(text, edited);
    assert!(ciq_compiler::parse_function_ir(&edited).is_err());
    for body in [
        "return flag?a:null;",
        "return a as Lang.Dictionary<Lang.String,Lang.Number>;",
    ] {
        let invalid = source.replace("return flag?a:b;", body);
        assert!(ciq_compiler::compile_function(&invalid).is_err());
    }
    let invalid = source.replace(
        "b as Lang.Dictionary<Lang.String,Lang.Number>",
        "b as Lang.Dictionary<Lang.String,Lang.Object>",
    );
    assert!(ciq_compiler::compile_function(&invalid).is_err());
}

#[test]
fn field_literals_recheck_source_interface_payloads() {
    let source = "class Holder {var a as Lang.Array<Lang.Comparable>;var d as Lang.Dictionary<Lang.Comparable,Lang.Comparable>;function initialize(){self.a=[];self.d={};}function fill() as Void {self.a=[new Item()];self.d={new Item()=>new Item()};}}";
    let item = "class Item {function initialize(){} function compareTo(other as Lang.Object) as Lang.Number{return 0;}}";
    let unit = compile_sources(&[source, item]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let broken = item.replace("other as Lang.Object", "other as Lang.Number");
    assert!(compile_sources(&[source, &broken]).is_err());
    assert!(
        parse_unit_ir(
            &unit
                .to_ir_text()
                .replace("parameter-types=object", "parameter-types=integer")
        )
        .is_err()
    );
}

#[test]
fn object_field_assignments_keep_enum_nominality_and_reject_void() {
    let source = "using Toybox.Graphics;class C {var value as Lang.Object;function initialize(){self.value=Lang.NUMBER_FORMAT_UINT8;}function write() as Void{self.value=Lang.ENDIAN_BIG;self.value=Graphics.FONT_MEDIUM;}}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert!(compile_sources(&["class C {var value as Lang.Number;function initialize(){self.value=Lang.NUMBER_FORMAT_UINT8;}}"]).is_err());
    assert!(compile_sources(&["class C {var value as Lang.Object;function initialize(){self.value=1;}function nothing() as Void{}function bad() as Void{self.value=self.nothing();}}"]).is_err());
}
