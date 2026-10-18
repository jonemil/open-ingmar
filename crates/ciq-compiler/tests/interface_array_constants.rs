use ciq_compiler::{
    ArrayConstantValue, Function, Operation,
    api::{ApiContext, ApiLevel},
    compile, parse_ir, parse_ir_with_api,
};

#[test]
fn tagged_constants_preserve_vm_types_pool_text_and_saved_stages() {
    let source = "function f() as Toybox.Lang.Array<Toybox.Lang.Comparable> {return [97,'a',true,1l,1.0,1.0d,\"line\\n\\\"quoted\\\"\",\"line\\n\\\"quoted\\\"\"]; }";
    let function = compile(source).unwrap();
    let text = function.to_ir_text();
    assert!(text.contains("comparable number 0x00000061 char 0x00000061 boolean true"));
    assert_eq!(function, parse_ir(&text).unwrap());
    let assembly = function.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    let arrays = assembly
        .items()
        .iter()
        .filter_map(|item| match item {
            ciq_assembly::Item::Array(array) => Some(array),
            _ => None,
        })
        .collect::<Vec<_>>();
    let [array] = arrays.as_slice() else {
        panic!("expected one constant Array")
    };
    use ciq_prg::data::arrays::Value;
    assert_eq!(
        &array.values[..6],
        &[
            Value::Number(97),
            Value::Char(97),
            Value::Boolean(true),
            Value::Long(1),
            Value::Float(1.0f32.to_bits()),
            Value::Double(1.0f64.to_bits())
        ]
    );
    let pool = array.pool.as_ref().unwrap();
    assert_eq!(pool.len(), 1);
    assert_eq!(pool[0].value, "line\n\"quoted\"");
    assert_eq!(array.values[6], array.values[7]);
    assert_eq!(array.values[6], Value::String(pool[0].alias.clone()));
}

#[test]
fn constants_validate_interface_witnesses_nullability_and_api_levels() {
    let text = "ciq-ir\n.function f\n.array-constant %0 comparable number 0x00000061 char 0x00000061 float 0x80000000 double 0x7ff8000000001234\n.return %0\n.end\n";
    let function = parse_ir(text).unwrap();
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    let values = function
        .operations()
        .iter()
        .find_map(|op| match &op.operation {
            Operation::ArrayConstant { values, .. } => Some(values),
            _ => None,
        })
        .unwrap();
    assert_eq!(values[2], ArrayConstantValue::Float(0x80000000));
    assert_eq!(values[3], ArrayConstantValue::Double(0x7ff8000000001234));
    let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap();
    assert!(parse_ir_with_api(text, context).is_ok());
    let low = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(4, 2, 0)).unwrap();
    assert!(parse_ir_with_api(text, low).is_err());
    for payload in [
        "number",
        "number 0x100000000",
        "float 0x100000000",
        "double 0x10000000000000000",
        "string unquoted",
        "boolean 1",
        "object 0x00000001",
        "0x00000061",
        "null",
    ] {
        let edited = text.replace(
            "number 0x00000061 char 0x00000061 float 0x80000000 double 0x7ff8000000001234",
            payload,
        );
        assert!(parse_ir(&edited).is_err(), "{edited}");
    }
    for tag in ["comparable", "comparator"] {
        let text = format!(
            "ciq-ir\n.function f\n.array-constant %0 nullable-{tag} null null\n.return %0\n.end\n"
        );
        let function = parse_ir(&text).unwrap();
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
        assert!(parse_ir(&text.replace(&format!("nullable-{tag}"), tag)).is_err());
    }
    assert!(parse_ir(&text.replace("comparable", "comparator")).is_err());
}

#[test]
fn typed_payloads_cannot_retag_number_as_char_or_replace_computed_objects() {
    for (ty, literal, wrong) in [
        ("Number", "97", ArrayConstantValue::Char(97)),
        ("Char", "'a'", ArrayConstantValue::Number(97)),
    ] {
        let function = compile(&format!(
            "function f() as Toybox.Lang.Array<Toybox.Lang.{ty}> {{return [{literal}];}}"
        ))
        .unwrap();
        let mut operations = function.operations().to_vec();
        let Operation::ArrayConstant { values, .. } = &mut operations[0].operation else {
            panic!("expected constant")
        };
        values[0] = wrong;
        assert!(Function::new("bad".into(), operations).is_err());
    }
    let source = "module M {function value() as Toybox.Lang.Comparable{return 1;} function f() as Toybox.Lang.Array<Toybox.Lang.Comparable> {return [M.value(),true ? M.value() : M.value()];}}";
    let unit = ciq_compiler::compile_sources(&[source]).unwrap();
    assert!(unit.to_ir_text().contains(".array-begin"));
    assert!(!unit.to_ir_text().contains(".array-constant"));
    assert_eq!(
        unit,
        ciq_compiler::parse_unit_ir(&unit.to_ir_text()).unwrap()
    );
}
