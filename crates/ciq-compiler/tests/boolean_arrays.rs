use ciq_compiler::{
    ArrayConstantValue, ArrayElement, Function, Operation, ReturnType,
    api::{ApiContext, ApiLevel},
    compile, compile_sources, compile_with_api, parse_ir, parse_ir_with_api, parse_unit_ir,
};

#[test]
fn boolean_arrays_preserve_types_and_native_saved_stages() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-boolean-arrays/app.mc"
    )])
    .unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let p = unit.lower();
    assert_eq!(p, ciq_assembly::Program::parse(&p.to_text()).unwrap());
    let f =
        compile("function f() as Toybox.Lang.Array<Toybox.Lang.Boolean> { return [true, false]; }")
            .unwrap();
    assert_eq!(f.return_type(), ReturnType::Array(ArrayElement::Boolean));
    let text = f.to_ir_text();
    assert!(text.contains(".array-constant %0 boolean true false"));
    assert_eq!(f, parse_ir(&text).unwrap());
    let mut bad = f.operations().to_vec();
    let Operation::ArrayConstant { values, .. } = &mut bad[0].operation else {
        unreachable!()
    };
    values[0] = ArrayConstantValue::Bits(1);
    assert!(Function::new("bad".into(), bad).is_err());
    for values in ["null", "0x00000001", "true 1", "\"true\"", "True"] {
        assert!(
            parse_ir(&text.replace("boolean true false", &format!("boolean {values}"))).is_err()
        );
    }
}

#[test]
fn boolean_element_contracts_do_not_convert_numbers_or_allow_null() {
    for source in [
        "function f() { var a = [true, 1]; }",
        "function f() { var a = [true, null]; }",
        "function f() { var a = [true]; a[0] = 1; }",
        "function f() { var a = [true]; a.add(null); }",
        "function f() { var a = [true]; a.addAll([1]); }",
        "function f() { var a = [true]; a[0]++; }",
        "function f() { var a = [true]; return a[0] + 1; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Boolean or Null>) { return a.size(); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Boolean>) as Toybox.Lang.Array<Toybox.Lang.Number> { return a; }",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    let f = compile("function f(a as Toybox.Lang.Array<Toybox.Lang.Boolean>, b as Toybox.Lang.Boolean) as Toybox.Lang.Boolean { a[0] = b; return !a[0]; }").unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    assert!(
        parse_ir(
            &f.to_ir_text()
                .replace("boolean-array:boolean", "boolean-array:integer")
        )
        .is_err()
    );
}

#[test]
fn boolean_construction_obeys_api_and_builder_frame_limits() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    for source in [
        "function f() as Toybox.Lang.Array<Toybox.Lang.Boolean> { return []; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Boolean> { return [true, false]; }",
        "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.Array<Toybox.Lang.Boolean> { return [b, !b]; }",
    ] {
        let f = compile_with_api(source, initial).unwrap();
        assert_eq!(f, parse_ir_with_api(&f.to_ir_text(), initial).unwrap());
    }
    let f = compile(
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Boolean>) { a.add(true); return a.size(); }",
    )
    .unwrap();
    assert!(parse_ir_with_api(&f.to_ir_text(), initial).is_err());
    let text = "ciq-ir\n.function f parameters=254\n.array-begin &0 boolean 1\n.boolean %0 true\n.array-fill &0 0 %0\n.array-finish %1 &0\n.return %1\n.end\n";
    assert_eq!(
        &parse_ir(text).unwrap().lower().encode().unwrap()[..3],
        &[0x4a, 255, 1]
    );
    assert!(parse_ir(&text.replace("parameters=254", "parameters=255")).is_err());
    assert!(parse_ir(&text.replace(".boolean %0 true", ".integer %0 1")).is_err());
}
