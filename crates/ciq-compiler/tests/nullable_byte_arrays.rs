use ciq_compiler::{
    ParameterType, ReturnType,
    api::{ApiContext, ApiLevel},
    compile, compile_sources, compile_with_api, parse_ir, parse_ir_with_api, parse_unit_ir,
};

#[test]
fn nullable_byte_arrays_preserve_signatures_and_local_null_proofs() {
    for source in [
        "function f(b as Toybox.Lang.ByteArray or Null) as Toybox.Lang.ByteArray or Null {return b;}",
        "function f() as Toybox.Lang.ByteArray or Null {return null;}",
        "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.ByteArray or Null {return b?[7]b:null;}",
        "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.ByteArray or Null {return b?null:[7]b;}",
        "function f(b as Toybox.Lang.ByteArray or Null,n as Toybox.Lang.ByteArray) as Void {var local=b;local=n;local=null;}",
        "function f(b as Toybox.Lang.ByteArray or Null) {if(null==b){return 0;}b[0]='A';b[0]+=1;return b[0];}",
        "function f(b as Toybox.Lang.ByteArray or Null) as Toybox.Lang.Boolean {return b!=null && b.size()>0;}",
        "function f(b as Toybox.Lang.ByteArray or Null) {return null!=b?b.size():0;}",
        "function f(b as Toybox.Lang.ByteArray or Null) {var local=b;return local!=null?local[0]:0;}",
        "function f(b as Toybox.Lang.ByteArray or Null) as Void {var local=b;while(local!=null){local.add(7);local=null;}}",
        "function f(b as Toybox.Lang.ByteArray or Null) as Void {if(b!=null){if(b!=null){b.size();}}}",
        "function f(b as Toybox.Lang.ByteArray,c as Toybox.Lang.ByteArray or Null) as Toybox.Lang.Boolean {return b.equals(c);}",
        "function f(b as Toybox.Lang.ByteArray or Null) as Toybox.Lang.Object or Null {return b;}",
    ] {
        let f = compile(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    let f = compile("function f(b as Toybox.Lang.ByteArray or Null) as Toybox.Lang.ByteArray or Null {return b;}").unwrap();
    assert_eq!(f.parameter_types(), &[ParameterType::NullableByteArray]);
    assert_eq!(f.return_type(), ReturnType::NullableByteArray);
    assert!(
        parse_ir(
            &f.to_ir_text()
                .replace("result=nullable-byte-array", "result=byte-array")
        )
        .is_err()
    );
    let f =
        compile("function f(b as Toybox.Lang.ByteArray or Null) as Void {if(b!=null){b.size();}}")
            .unwrap();
    assert!(parse_ir(&f.to_ir_text().replace(".not-equal ", ".equal ")).is_err());
}

#[test]
fn nullable_byte_array_uses_require_current_nonnull_proofs() {
    for source in [
        "function f(b as Toybox.Lang.ByteArray or Null) {return b.size();}",
        "function f(b as Toybox.Lang.ByteArray or Null) {return b[0];}",
        "function f(b as Toybox.Lang.ByteArray or Null) {return true?b[0]:0;}",
        "function f(b as Toybox.Lang.ByteArray or Null) as Void {b[0]=7;}",
        "function f(b as Toybox.Lang.ByteArray or Null) as Void {b[0]+=7;}",
        "function f(b as Toybox.Lang.ByteArray or Null) as Toybox.Lang.ByteArray {return b;}",
        "function f(b as Toybox.Lang.ByteArray or Null) as Void {var local=b;if(local!=null){local=null;local.size();}}",
        "function f(b as Toybox.Lang.ByteArray or Null) as Void {var local=b;if(b!=null){local.size();}}",
        "function f(b as Toybox.Lang.ByteArray or Null) as Void {var present=b!=null;if(present){b.size();}}",
        "function f(b as Toybox.Lang.ByteArray or Null) as Void {var local=b;local=[1];}",
        "function f(b as Toybox.Lang.Object) {return b as Toybox.Lang.ByteArray;}",
        "function f(b as Toybox.Lang.Object or Null) {return b as Toybox.Lang.ByteArray or Null;}",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    let invalid = "module B {function size(b as Toybox.Lang.ByteArray){return b.size();} function f(b as Toybox.Lang.ByteArray or Null){return B.size(b);}}";
    assert!(compile_sources(&[invalid]).is_err());
    let valid = invalid.replace(
        "return B.size(b);",
        "if(b==null){return 0;}return B.size(b);",
    );
    let unit = compile_sources(&[&valid]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn nullable_byte_array_declarations_require_api_three_even_without_allocation() {
    let modern = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(3, 0, 0)).unwrap();
    let old = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(2, 4, 0)).unwrap();
    for source in [
        "function f(b as Toybox.Lang.ByteArray or Null) as Void {}",
        "function f() as Toybox.Lang.ByteArray or Null {return null;}",
    ] {
        let f = compile_with_api(source, modern).unwrap();
        assert_eq!(f, parse_ir_with_api(&f.to_ir_text(), modern).unwrap());
        assert!(compile_with_api(source, old).is_err(), "{source}");
        assert!(parse_ir_with_api(&f.to_ir_text(), old).is_err());
    }
}
