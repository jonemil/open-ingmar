#[path = "common/checking.rs"]
mod checking;
use ciq_assembly::{Item, Program};
use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_function_with_api, compile_sources, parse_function_ir,
    parse_function_ir_with_api, parse_unit_ir,
};

#[test]
fn numeric_array_group_preserves_all_saved_stages_and_application_bytes() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-numeric-arrays/app.mc"
    )])
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [94; 16],
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
fn numeric_array_literals_bind_by_exact_bits_and_type() {
    for (ty, values, tag) in [
        ("Long", "0l, 4294967297l, -1l", "long"),
        ("Float", "0.0, -0.0, 1.25", "float"),
        ("Double", "0.0d, -0.0d, 16777217.0d", "double"),
    ] {
        let f = compile_function(&format!(
            "function f() as Toybox.Lang.Array<Toybox.Lang.{ty}> {{ return [{values}]; }}"
        ))
        .unwrap();
        assert!(
            f.to_ir_text()
                .contains(&format!(".array-constant %0 {tag}"))
        );
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
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
        assert_eq!(ciq_compiler::array_bindings(&asm, &data).unwrap().len(), 1);
        let mut wrong = record.clone();
        wrong.values[0] = ciq_prg::data::arrays::Value::Number(0);
        let wrong = ciq_prg::data::Source::parse(&wrong.to_text()).unwrap();
        assert!(ciq_compiler::array_bindings(&asm, &wrong).is_err());
        if tag != "long" {
            let mut wrong = record;
            wrong.values.swap(0, 1);
            let wrong = ciq_prg::data::Source::parse(&wrong.to_text()).unwrap();
            assert!(ciq_compiler::array_bindings(&asm, &wrong).is_err());
        }
        let ty_lower = ty.to_lowercase();
        let width = if tag == "float" { 8 } else { 16 };
        let bits = if tag == "float" {
            "7fc12345"
        } else {
            "7ff8000000001234"
        };
        let text = format!(
            "ciq-ir\n.function f\n.array-constant %0 {ty_lower} 0x{bits}\n.return %0\n.end\n"
        );
        let ir = parse_function_ir(&text).unwrap();
        assert_eq!(ir, parse_function_ir(&ir.to_ir_text()).unwrap());
        for bad in ["0x1", "-1", "0xgggggggg", "0x10000000000000000"] {
            assert!(
                parse_function_ir(&text.replace(&format!("0x{bits}"), bad)).is_err(),
                "{width}: {bad}"
            );
        }
    }
}

#[test]
fn new_array_types_share_nullable_and_invariant_contracts() {
    for (ty, literal, other) in [
        ("Long", "1l", "1"),
        ("Float", "1.25", "1.25d"),
        ("Double", "1.25d", "1.25"),
    ] {
        for body in [
            format!("return [{literal}, null];"),
            "return [];".into(),
            format!("return [true ? {literal} : null];"),
        ] {
            let f = compile_function(&format!(
                "function f() as Toybox.Lang.Array<Toybox.Lang.{ty} or Null> {{ {body} }}"
            ))
            .unwrap();
            assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        }
        for body in [
            "return a[0];".to_owned(),
            "var n=a[0]; if (n == null) { return ZERO; } a[0]=null; return n;"
                .replace("ZERO", literal),
        ] {
            let nullable = body == "return a[0];";
            let f=compile_function(&format!("function f(a as Toybox.Lang.Array<Toybox.Lang.{ty} or Null>) as Toybox.Lang.{ty}{} {{ {body} }}",if nullable {" or Null"}else{""})).unwrap();
            assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        }
        for body in [
            format!("a.add({other});"),
            format!("a[0]={other};"),
            format!("a.addAll([{other}]);"),
            "a.add(null);".into(),
            "a[1l]=a[0];".into(),
        ] {
            assert!(
                checking::compile_function(&format!(
                    "function f(a as Toybox.Lang.Array<Toybox.Lang.{ty}>) {{ {body} }}"
                ))
                .is_err(),
                "{ty}: {body}"
            );
        }
        assert!(checking::compile_function(&format!("function f(a as Toybox.Lang.Array<Toybox.Lang.{ty} or Null>) as Toybox.Lang.{ty} {{ return a[0]; }}")).is_err());
        assert!(checking::compile_function(&format!("function f(a as Toybox.Lang.Array<Toybox.Lang.{ty} or Null>) as Toybox.Lang.{ty} {{ var n=a[0]; if(n == null) {{ return {literal}; }} return a[0]; }}")).is_err());
        let f=compile_function(&format!("function f() as Toybox.Lang.Array<Toybox.Lang.{ty}> {{ return [{literal} + {literal}]; }}")).unwrap();
        assert!(f.to_ir_text().contains(".array-begin"));
        assert!(!f.to_ir_text().contains(".array-constant"));
    }
}

#[test]
fn generic_native_api_gates_are_preserved_for_new_elements() {
    for (ty, literal) in [("Long", "1l"), ("Float", "1.25"), ("Double", "1.25d")] {
        let source =
            format!("function f(a as Toybox.Lang.Array<Toybox.Lang.{ty}>) {{ return a.size(); }}");
        let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
        compile_function_with_api(&source, initial).unwrap();
        let source = format!(
            "function f(a as Toybox.Lang.Array<Toybox.Lang.{ty}>) {{ return a.indexOf({literal}); }}"
        );
        let f = compile_function(&source).unwrap();
        assert!(compile_function_with_api(&source, initial).is_err());
        assert!(parse_function_ir_with_api(&f.to_ir_text(), initial).is_err());
        let introduced = ApiContext::new(ApiLevel(1, 3, 0), ApiLevel(1, 3, 0)).unwrap();
        assert_eq!(
            compile_function_with_api(&source, introduced).unwrap(),
            parse_function_ir_with_api(&f.to_ir_text(), introduced).unwrap()
        );
    }
}

#[test]
fn native_constant_construction_rejects_a_mismatched_element_payload() {
    let f =
        compile_function("function f() as Toybox.Lang.Array<Toybox.Lang.Float> { return [1.25]; }")
            .unwrap();
    let mut operations = f.operations().to_vec();
    let ciq_compiler::Operation::ArrayConstant { element_type, .. } = &mut operations[0].operation
    else {
        panic!()
    };
    *element_type = ciq_compiler::ArrayElement::Long;
    assert!(ciq_compiler::Function::new("bad".into(), operations).is_err());
}
