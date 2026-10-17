use ciq_assembly::{Item, Program};
use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, compile_with_api, parse_ir, parse_ir_with_api, parse_unit_ir,
};

#[test]
fn numeric_dictionary_group_preserves_all_saved_stages() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-numeric-dictionaries/app.mc"
    )])
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [95; 16],
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
fn all_numeric_key_value_contracts_are_invariant_and_nullable_values_survive_ir() {
    for (key, k) in [
        ("Long", "4294967297l"),
        ("Float", "1.25"),
        ("Double", "1.25d"),
    ] {
        for (value, v) in [
            ("Long", "4294967297l"),
            ("Float", "-0.0"),
            ("Double", "16777217.0d"),
        ] {
            for nullable in [false, true] {
                let suffix = if nullable { " or Null" } else { "" };
                let source = format!(
                    "function f() as Toybox.Lang.Dictionary<Toybox.Lang.{key}, Toybox.Lang.{value}{suffix}> {{ return {{{k} => {v}}}; }}"
                );
                let f = compile(&source).unwrap();
                assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
                assert!(f.to_ir_text().contains(".dictionary-constant"));
                let asm = f.lower();
                let record = asm
                    .items()
                    .iter()
                    .find_map(|i| {
                        if let Item::Dictionary(d) = i {
                            Some(d.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap();
                let source = ciq_prg::data::Source::parse(&record.to_text()).unwrap();
                assert!(ciq_compiler::dictionary_bindings(&asm, &source).is_ok());
                let mut wrong = record;
                wrong.entries[0].1 = ciq_prg::data::dictionaries::Value::Number(1);
                let source = ciq_prg::data::Source::parse(&wrong.to_text()).unwrap();
                assert!(ciq_compiler::dictionary_bindings(&asm, &source).is_err());
            }
            let declaration = format!(
                "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.{key},Toybox.Lang.{value} or Null>) as Toybox.Lang.{value} or Null {{ return d[{k}]; }}"
            );
            let f = compile(&declaration).unwrap();
            assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
            for body in [
                format!("d.put({k},1);"),
                format!("d.put(1,{v});"),
                format!("d[1]={v};"),
            ] {
                assert!(compile(&format!("function f(d as Toybox.Lang.Dictionary<Toybox.Lang.{key},Toybox.Lang.{value}>) {{ {body} }}")).is_err(),"{key}, {value}: {body}");
            }
        }
    }
    for (ty, v) in [("Long", "1l"), ("Float", "1.25"), ("Double", "1.25d")] {
        for body in [
            format!("return {{1 => {v}, 2 => null}};"),
            "return {1 => null};".into(),
            "return {};".into(),
            format!("return {{1 => true ? {v} : null}};"),
        ] {
            let f=compile(&format!("function f() as Toybox.Lang.Dictionary<Toybox.Lang.Number,Toybox.Lang.{ty} or Null> {{ {body} }}")).unwrap();
            assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        }
        for body in [
            "return d[1];".into(),
            format!("var n=d[1]; if(n == null) {{ return {v}; }} return d[1];"),
        ] {
            assert!(compile(&format!("function f(d as Toybox.Lang.Dictionary<Toybox.Lang.Number,Toybox.Lang.{ty}>) as Toybox.Lang.{ty} {{ {body} }}")).is_err());
        }
    }
}

#[test]
fn key_canonicalization_retains_signed_zero_and_double_nan_entries() {
    for (ty, zero) in [("float", "00000000"), ("double", "0000000000000000")] {
        let negative = if ty == "float" {
            "80000000"
        } else {
            "8000000000000000"
        };
        let text = format!(
            "ciq-ir\n.function f\n.dictionary-constant %0 dictionary({ty}=>integer) {ty} 0x{zero} number 0x00000001 {ty} 0x{negative} number 0x00000002 {ty} 0x{zero} number 0x00000003\n.return %0\n.end\n"
        );
        let f = parse_ir(&text).unwrap();
        let asm = f.lower();
        let record = asm
            .items()
            .iter()
            .find_map(|i| {
                if let Item::Dictionary(d) = i {
                    Some(d)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(record.entries.len(), 2);
        assert_eq!(
            record.entries[0].1,
            ciq_prg::data::dictionaries::Value::Number(3)
        );
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    for (ty, bits, count) in [("float", "7fc00000", 1), ("double", "7ff8000000000000", 2)] {
        let text = format!(
            "ciq-ir\n.function f\n.dictionary-constant %0 dictionary({ty}=>integer) {ty} 0x{bits} number 0x00000001 {ty} 0x{bits} number 0x00000002\n.return %0\n.end\n"
        );
        let f = parse_ir(&text).unwrap();
        let asm = f.lower();
        let record = asm
            .items()
            .iter()
            .find_map(|i| {
                if let Item::Dictionary(d) = i {
                    Some(d)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(record.entries.len(), count);
    }
}

#[test]
fn malformed_constants_and_unavailable_methods_are_rejected() {
    for (ty, literal) in [("Long", "1l"), ("Float", "1.25"), ("Double", "1.25d")] {
        let source = format!(
            "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.{ty},Toybox.Lang.{ty}>) as Toybox.Lang.{ty} or Null {{ return d.get({literal}); }}"
        );
        let context = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
        let f = compile_with_api(&source, context).unwrap();
        assert_eq!(f, parse_ir_with_api(&f.to_ir_text(), context).unwrap());
        assert!(compile(&format!("function f(d as Toybox.Lang.Dictionary<Toybox.Lang.{ty},Toybox.Lang.{ty}>) {{ d.put({literal},null); }}")).is_err());
        let f=compile(&format!("function f() as Toybox.Lang.Dictionary<Toybox.Lang.{ty},Toybox.Lang.{ty}> {{ return {{{literal}=>{literal}}}; }}")).unwrap();
        let text = f.to_ir_text();
        for bad in ["0xgg", "0x10000000000000000"] {
            let offset = text.find(" 0x").unwrap() + 1;
            let end = text[offset..]
                .find(char::is_whitespace)
                .map(|n| offset + n)
                .unwrap();
            assert!(parse_ir(&format!("{}{}{}", &text[..offset], bad, &text[end..])).is_err());
        }
        let to_string = format!(
            "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.{ty},Toybox.Lang.{ty}>) as Toybox.Lang.String {{ return d.toString(); }}"
        );
        assert!(compile_with_api(&to_string, context).is_err());
    }
}
