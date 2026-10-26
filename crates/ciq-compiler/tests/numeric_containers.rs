#[path = "common/checking.rs"]
mod checking;
use ciq_assembly::{Item, Program};
use ciq_compiler::{
    ArrayElement, DictionaryKey, DictionaryType, DictionaryValue, NumericAlias,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources_with_api, parse_function_ir, parse_unit_ir,
};

#[test]
fn numeric_container_application_preserves_saved_stages() {
    let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[include_str!(
            "../../../examples/compiler-numeric-containers/app.mc"
        )],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [96; 16],
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
fn alias_array_constants_retain_scalar_tags_and_exact_bits() {
    for (alias, name, tag, values) in [
        (
            NumericAlias::Integer,
            "Integer",
            "integer-union",
            "1, 4294967297l",
        ),
        (
            NumericAlias::Decimal,
            "Decimal",
            "decimal",
            "-0.0, 16777217.0d",
        ),
        (
            NumericAlias::Numeric,
            "Numeric",
            "numeric",
            "1, 4294967297l, -0.0, 16777217.0d",
        ),
    ] {
        for nullable in [false, true] {
            let suffix = if nullable { " or Null" } else { "" };
            let null = if nullable { ", null" } else { "" };
            let source = format!(
                "function f() as Lang.Array<Lang.{name}{suffix}> {{return [{values}{null}];}}"
            );
            let f = compile_function(&source).unwrap();
            assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
            assert!(f.to_ir_text().contains(&format!(
                "{}{} ",
                if nullable { "nullable-" } else { "" },
                tag
            )));
            let asm = f.lower();
            let record = asm
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
            let mut changed = record.clone();
            changed.values.swap(0, 1);
            let data = ciq_prg::data::Source::parse(&changed.to_text()).unwrap();
            assert!(ciq_compiler::array_bindings(&asm, &data).is_err());
            assert_eq!(
                f.return_type(),
                ciq_compiler::ReturnType::Array(ArrayElement::Numeric { alias, nullable })
            );
            let edited = f.to_ir_text().replace(&format!("{tag} "), "number ");
            assert_ne!(edited, f.to_ir_text());
            assert!(parse_function_ir(&edited).is_err());
        }
    }
}

#[test]
fn dictionary_alias_keys_values_and_enumerations_keep_invariant_types() {
    for (alias, name, literal) in [
        (NumericAlias::Integer, "Integer", "1l"),
        (NumericAlias::Decimal, "Decimal", "1.5d"),
        (NumericAlias::Numeric, "Numeric", "1.5"),
    ] {
        for nullable in [false, true] {
            let suffix = if nullable { " or Null" } else { "" };
            let source = format!(
                "function f() as Lang.Dictionary<Lang.{name},Lang.{name}{suffix}> {{return {{{literal}=>{literal}}};}}"
            );
            let f = compile_function(&source).unwrap();
            assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
            assert_eq!(
                f.return_type(),
                ciq_compiler::ReturnType::TypedDictionary(DictionaryType {
                    key: DictionaryKey::Numeric(alias),
                    value: DictionaryValue::Numeric { alias, nullable }
                })
            );
            let asm = f.lower();
            let mut record = asm
                .items()
                .iter()
                .find_map(|item| {
                    if let Item::Dictionary(d) = item {
                        Some(d.clone())
                    } else {
                        None
                    }
                })
                .unwrap();
            let data = ciq_prg::data::Source::parse(&record.to_text()).unwrap();
            assert!(ciq_compiler::dictionary_bindings(&asm, &data).is_ok());
            record.entries[0].1 = ciq_prg::data::dictionaries::Value::Number(99);
            let data = ciq_prg::data::Source::parse(&record.to_text()).unwrap();
            assert!(ciq_compiler::dictionary_bindings(&asm, &data).is_err());
            for body in [
                format!(
                    "function f(d as Lang.Dictionary<Lang.{name},Lang.{name}{suffix}>) as Lang.Array<Lang.{name}{suffix}> {{return d.values();}}"
                ),
                format!(
                    "function f(d as Lang.Dictionary<Lang.{name},Lang.{name}{suffix}>) as Lang.Array<Lang.{name}> {{return d.keys();}}"
                ),
                format!(
                    "function f(n as Lang.{name}) as Lang.Dictionary<Lang.{name},Lang.{name}> {{var d={{n=>n}}; return d;}}"
                ),
                format!(
                    "function f(n as Lang.{name} or Null) as Lang.Dictionary<Lang.String,Lang.{name} or Null> {{var d={{\"n\"=>n}}; return d;}}"
                ),
            ] {
                let f = compile_function(&body).unwrap_or_else(|e| panic!("{body}: {e}"));
                assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
            }
        }
    }
}

#[test]
fn nullable_alias_elements_need_snapshot_proofs_and_allocations_stay_nullable() {
    for name in ["Integer", "Decimal", "Numeric"] {
        for source in [
            format!(
                "function f(a as Lang.Array<Lang.{name} or Null>) as Lang.{name} {{var n=a[0];if(n!=null){{a[0]=null;return n+1;}}return 0 as Lang.{name};}}"
            ),
            format!("function f(n) as Lang.Array<Lang.{name} or Null> {{return new[n];}}"),
            format!(
                "function f(n as Lang.{name}) as Lang.Array<Lang.{name}> {{var a=[n,n];return a;}}"
            ),
            format!(
                "function f(a as Lang.Array<Lang.{name}>,flag as Lang.Boolean) as Lang.Array<Lang.{name}> {{return flag?a:a;}}"
            ),
            format!(
                "function f(n as Lang.{name} or Null) as Lang.Dictionary<Lang.{name},Lang.{name} or Null> {{if(n!=null){{var d={{n=>n}};return d;}}return {{}};}}"
            ),
        ] {
            let f = compile_function(&source).unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
            if f.to_ir_text().contains(".not-equal ") {
                assert!(
                    parse_function_ir(&f.to_ir_text().replace(".not-equal ", ".equal ")).is_err()
                );
            }
        }
        for source in [
            format!(
                "function f(a as Lang.Array<Lang.{name} or Null>) as Lang.{name} {{return a[0]+1;}}"
            ),
            format!(
                "function f(a as Lang.Array<Lang.{name} or Null>) as Lang.{name} {{if(a[0]!=null){{return a[0]+1;}}return 0 as Lang.{name};}}"
            ),
            format!("function f(a as Lang.Array<Lang.{name}>) as Void {{a.add(null);}}"),
            format!("function f(n) as Lang.Array<Lang.{name}> {{return new[n];}}"),
            format!(
                "function f(d as Lang.Dictionary<Lang.{name},Lang.{name}>) as Lang.{name} {{return d[1 as Lang.{name}]+1;}}"
            ),
            format!(
                "function f(d as Lang.Dictionary<Lang.{name},Lang.{name}>) as Void {{d.put(null,1);}}"
            ),
            format!(
                "function f(n as Lang.{name} or Null) as Lang.Dictionary<Lang.{name},Lang.{name} or Null> {{var d={{n=>n}};return d;}}"
            ),
            format!(
                "function f() as Lang.Dictionary<Lang.{name} or Null,Lang.{name}> {{return {{}};}}"
            ),
        ] {
            assert!(checking::compile_function(&source).is_err(), "{source}");
        }
    }
}

#[test]
fn alias_containers_reject_covariance_and_unsupported_scalar_alternatives() {
    for source in [
        "function f(a as Lang.Array<Lang.Number>) as Lang.Array<Lang.Integer> {return a;}",
        "function f(a as Lang.Array<Lang.Integer>) as Lang.Array<Lang.Numeric> {return a;}",
        "function f(a as Lang.Array<Lang.Numeric>) as Void {a.addAll([true]);}",
        "function f() as Lang.Array<Lang.Integer> {return [1,1.5];}",
        "function f() as Lang.Array<Lang.Decimal> {return [1.5,1l];}",
        "function f(a as Lang.Array<Lang.Numeric or Null>) as Void {a.sort(null);}",
        "function f(d as Lang.Dictionary<Lang.Number,Lang.Number>) as Lang.Dictionary<Lang.Integer,Lang.Integer> {return d;}",
        "function f(d as Lang.Dictionary<Lang.Integer,Lang.Integer>) as Void {d.put(1.5,1);}",
        "function f(d as Lang.Dictionary<Lang.Decimal,Lang.Decimal>) as Void {d.put(1.5,1);}",
        "function f(d as Lang.Dictionary<Lang.Numeric,Lang.Numeric>) as Void {d.put('A',1);}",
    ] {
        assert!(checking::compile_function(source).is_err(), "{source}");
    }
}
