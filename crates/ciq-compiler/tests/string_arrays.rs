use ciq_compiler::{
    ArrayConstantValue, ArrayElement, Operation,
    api::{ApiContext, ApiLevel},
    compile, compile_sources, compile_with_api, parse_ir, parse_ir_with_api, parse_unit_ir,
};

#[test]
fn strings_preserve_types_and_pooled_constants_through_saved_stages() {
    let source = include_str!("../../../examples/compiler-string-arrays/app.mc");
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    let pooled = assembly
        .items()
        .iter()
        .find_map(|item| match item {
            ciq_assembly::Item::Array(a) if a.values.len() == 5 => Some(a),
            _ => None,
        })
        .unwrap();
    assert_eq!(pooled.pool.as_ref().unwrap().len(), 4);
    assert_eq!(pooled.values[0], pooled.values[2]);
    assert!(
        pooled
            .pool
            .as_ref()
            .unwrap()
            .iter()
            .any(|s| s.value.is_empty())
    );
    let f = compile(r#"function f() as Toybox.Lang.Array<Toybox.Lang.String> { return ["", "é", "quote\"slash\\\n"]; }"#).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    assert_eq!(
        f.return_type(),
        ciq_compiler::ReturnType::Array(ArrayElement::String)
    );
    assert!(
        matches!(&f.operations()[0].operation, Operation::ArrayConstant { values, .. } if matches!(&values[0], ArrayConstantValue::String(s) if s.is_empty()))
    );
    let mut malformed = f.operations().to_vec();
    let Operation::ArrayConstant { values, .. } = &mut malformed[0].operation else {
        unreachable!()
    };
    values[0] = ArrayConstantValue::Bits(41);
    assert!(ciq_compiler::Function::new("bad".into(), malformed).is_err());
}

#[test]
fn computed_strings_and_guarded_values_use_ordered_private_construction() {
    for source in [
        "function f(s as Toybox.Lang.String) as Toybox.Lang.Array<Toybox.Lang.String> { return [s, s.toUpper()]; }",
        "function f(s as Toybox.Lang.String or Null) as Toybox.Lang.Array<Toybox.Lang.String> { if (s == null) { return []; } return [s, s.toUpper()]; }",
        "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.Array<Toybox.Lang.String> { return [b ? \"yes\" : \"no\", \"tail\"]; }",
    ] {
        let f = compile(source).unwrap();
        let text = f.to_ir_text();
        assert!(text.contains(".array-begin"));
        assert!(text.contains(" string "));
        assert_eq!(f, parse_ir(&text).unwrap());
        assert!(parse_ir(&text.replace(" string ", " number ")).is_err());
    }
}

#[test]
fn invariant_non_null_string_elements_are_checked_in_source_and_ir() {
    for source in [
        "function f() { var a = [\"one\", 1]; }",
        "function f() { var a = [\"one\", null]; }",
        "function f() { var a = [[\"one\"]]; }",
        "function f(s as Toybox.Lang.String or Null) { var a = [s]; }",
        "function f() { var a = [\"one\"]; a[0] = null; }",
        "function f() { var a = [\"one\"]; a.add(1); }",
        "function f() { var a = [\"one\"]; a.addAll([1]); }",
        "function f() { var a = [\"one\"]; a[0]++; }",
        "function f() { var a = [\"one\"]; System.println(a); }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number> { return [\"one\"]; }",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    let f = compile("function f(a as Toybox.Lang.Array<Toybox.Lang.String>, s as Toybox.Lang.String) as Toybox.Lang.String { a[0] = s; return a[0]; }").unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    assert!(
        parse_ir(&f.to_ir_text().replace(
            "parameter-types=string-array:string",
            "parameter-types=string-array:nullable-string"
        ))
        .is_err()
    );
    let text = "ciq-ir\n.function f\n.array-constant %0 string \"one\"\n.return %0\n.end\n";
    assert!(parse_ir(text).is_ok());
    for bad in [
        ".array-constant %0 string",
        ".array-constant %0 string one",
        ".array-constant %0 string 0x00000001",
        ".array-constant %0 number \"one\"",
        ".array-constant %0 string \"one\" null",
    ] {
        assert!(
            parse_ir(&text.replace(".array-constant %0 string \"one\"", bad)).is_err(),
            "accepted {bad}"
        );
    }
}

#[test]
fn array_type_availability_does_not_enable_later_methods() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    for source in [
        "function f() as Toybox.Lang.Array<Toybox.Lang.String> { return []; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.String> { return [\"one\"]; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.String>) as Toybox.Lang.String { return a[0]; }",
    ] {
        let f = compile_with_api(source, initial).unwrap();
        assert_eq!(f, parse_ir_with_api(&f.to_ir_text(), initial).unwrap());
    }
    let source = "function f(a as Toybox.Lang.Array<Toybox.Lang.String>) { a.add(\"one\"); return a.size(); }";
    let later = ApiContext::new(ApiLevel(1, 3, 0), ApiLevel(1, 3, 0)).unwrap();
    let f = compile_with_api(source, later).unwrap();
    assert!(compile_with_api(source, initial).is_err());
    assert!(parse_ir_with_api(&f.to_ir_text(), initial).is_err());
}
