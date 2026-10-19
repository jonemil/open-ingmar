use ciq_compiler::{
    ArrayElement, Operation, ParameterType, ReturnType,
    api::{ApiContext, ApiLevel},
    compile, compile_sources, compile_with_api, parse_ir, parse_ir_with_api, parse_unit_ir,
};

#[test]
fn byte_array_elements_preserve_invariant_types_and_all_construction_paths() {
    for (element, nullable) in [
        (ArrayElement::ByteArray, ""),
        (ArrayElement::NullableByteArray, " or Null"),
    ] {
        let source = format!(
            "function f(a as Toybox.Lang.Array<Toybox.Lang.ByteArray{nullable}>) as Toybox.Lang.Array<Toybox.Lang.ByteArray{nullable}> {{return a;}}"
        );
        let f = compile(&source).unwrap();
        assert_eq!(f.parameter_types(), &[ParameterType::Array(element)]);
        assert_eq!(f.return_type(), ReturnType::Array(element));
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    for source in [
        "function f() {var a=[[1]b];return a[0][0];}",
        "function f() as Toybox.Lang.Array<Toybox.Lang.ByteArray> {return [];}",
        "function f() as Toybox.Lang.Array<Toybox.Lang.ByteArray> {return [[1]b,[2]b];}",
        "function f(b as Toybox.Lang.ByteArray) as Toybox.Lang.Array<Toybox.Lang.ByteArray> {return [b,b];}",
        "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.Array<Toybox.Lang.ByteArray> {return [[1]b,b?[2]b:[3]b];}",
        "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.Array<Toybox.Lang.ByteArray or Null> {return [null,b?[2]b:null];}",
        "function f(n) as Toybox.Lang.Array<Toybox.Lang.ByteArray or Null> {return new[n];}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.ByteArray>,b as Toybox.Lang.Boolean) as Toybox.Lang.Array<Toybox.Lang.ByteArray> {return b?a:[];}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.ByteArray or Null>,b as Toybox.Lang.Boolean) as Toybox.Lang.Array<Toybox.Lang.ByteArray or Null> {return b?a:[];}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.ByteArray>) as Void {a[0]=[1]b;a.add([2]b);a.addAll([]);a.reverse();a.slice(null,null);}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.ByteArray or Null>) {var b=a[0];if(b!=null){a[0]=null;return b[0];}return 0;}",
    ] {
        let f = compile(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    let unit = compile_sources(&["module B {function echo(a as Toybox.Lang.Array<Toybox.Lang.ByteArray>) as Toybox.Lang.Array<Toybox.Lang.ByteArray> {return a;}function use() as Void {B.echo([[1]b]);}} "]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn byte_array_element_nullability_and_class_identity_cannot_be_widened() {
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.ByteArray>) as Void {a[0]=null;}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.ByteArray>) as Void {a.add([1]);}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.ByteArray>) as Void {a.addAll([[1]]);}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.ByteArray>) as Void {a.sort(null);}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.ByteArray>) as Toybox.Lang.Array<Toybox.Lang.ByteArray or Null> {return a;}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.ByteArray or Null>) as Toybox.Lang.Array<Toybox.Lang.ByteArray> {return a;}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.ByteArray or Null>) {return a[0][0];}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.ByteArray or Null>) {if(a[0]!=null){return a[0].size();}return 0;}",
        "function f(n) as Toybox.Lang.Array<Toybox.Lang.ByteArray> {return new[n];}",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    let f = compile("function f(a as Toybox.Lang.Array<Toybox.Lang.ByteArray or Null>) as Void {var b=a[0];if(b!=null){b.size();}}").unwrap();
    assert!(parse_ir(&f.to_ir_text().replace(".not-equal ", ".equal ")).is_err());
}

#[test]
fn null_only_byte_array_elements_keep_api_and_constant_payload_checks() {
    let modern = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(3, 0, 0)).unwrap();
    let old = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(2, 4, 0)).unwrap();
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.ByteArray>) as Void {}",
        "function f() as Toybox.Lang.Array<Toybox.Lang.ByteArray or Null> {return [null];}",
    ] {
        let f = compile_with_api(source, modern).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        assert!(compile_with_api(source, old).is_err());
        assert!(parse_ir_with_api(&f.to_ir_text(), old).is_err());
        if source.contains("[null]") {
            assert!(
                f.operations()
                    .iter()
                    .any(|op| matches!(op.operation, Operation::ArrayConstant { .. }))
            );
            let text = f.to_ir_text();
            let edited = text.replace(" null span=", " 0x00000001 span=");
            assert_ne!(text, edited);
            assert!(parse_ir(&edited).is_err());
        }
    }
}
