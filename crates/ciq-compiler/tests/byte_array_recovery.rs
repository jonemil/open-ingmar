use ciq_compiler::{
    Operation,
    api::{ApiContext, ApiLevel},
    compile, compile_sources, compile_with_api, parse_ir, parse_ir_with_api, parse_unit_ir,
};

#[test]
fn byte_array_tests_recover_only_current_storage_reads_on_proven_paths() {
    for source in [
        "function f(o as Toybox.Lang.Object or Null) {if(o instanceof Lang.ByteArray){o[0]+=1;return o[0];}return -1;}",
        "function f(o as Toybox.Lang.Object or Null) {if(!(o instanceof Lang.ByteArray)){return -1;}return o.size();}",
        "function f(o as Toybox.Lang.Object or Null) as Toybox.Lang.Boolean {return o instanceof Lang.ByteArray && o.size()>0;}",
        "function f(o as Toybox.Lang.Object or Null) {return o instanceof Lang.ByteArray?o[0]:0;}",
        "function f(o as Toybox.Lang.Object or Null) as Void {var local=o;while(local instanceof Lang.ByteArray){local.add(1);local=null;}}",
        "function f(o as Toybox.Lang.Object or Null) as Toybox.Lang.ByteArray or Null {if(o instanceof Lang.ByteArray){return o;}return null;}",
        "using Toybox.Lang as L;function f(b as L.ByteArray or Null) {if(b instanceof L.ByteArray){return b.size();}return 0;}",
    ] {
        let f = compile(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        assert!(f.lower().to_text().contains("isa"));
    }
    for source in [
        "function f(o as Toybox.Lang.Object or Null) {if(o instanceof Lang.ByteArray){return 0;}return o.size();}",
        "function f(o as Toybox.Lang.Object or Null) {var local=o;if(local instanceof Lang.ByteArray){local=null;return local.size();}return 0;}",
        "function f(o as Toybox.Lang.Object or Null) {var copy=o;if(o instanceof Lang.ByteArray){return copy.size();}return 0;}",
        "function f(o as Toybox.Lang.Object or Null) {var check=o instanceof Lang.ByteArray;if(check){return o.size();}return 0;}",
        "function f(w as Toybox.Lang.WeakReference) {if(w.get() instanceof Lang.ByteArray){return w.get().size();}return 0;}",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    let f=compile("function f(o as Toybox.Lang.Object or Null) {if(o instanceof Lang.ByteArray){return o[0];}return 0;}").unwrap();
    let text = f.to_ir_text();
    assert!(parse_ir(&text.replace("Toybox.Lang.ByteArray", "Toybox.Lang.String")).is_err());
    let mut operations = f.operations().to_vec();
    for located in &mut operations {
        if let Operation::Branch {
            on_true, on_false, ..
        } = &mut located.operation
        {
            std::mem::swap(on_true, on_false);
        }
    }
    assert!(
        ciq_compiler::Function::with_parameter_types(
            "f".into(),
            f.parameter_types().to_vec(),
            operations
        )
        .is_err()
    );
}

#[test]
fn byte_array_assertions_are_unchecked_and_emit_no_conversion_or_test() {
    for suffix in ["", " or Null"] {
        let source = format!(
            "function f(o as Toybox.Lang.Object or Null) as Toybox.Lang.ByteArray{suffix} {{return o as Lang.ByteArray{suffix};}}"
        );
        let f = compile(&source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        assert_eq!(f.lower().encode().unwrap(), [0x35, 2, 0x12, 1, 0x16]);
    }
    let source = "module B {function take(b as Toybox.Lang.ByteArray){return b.size();}function f(o as Toybox.Lang.Object or Null){return B.take(o as Lang.ByteArray);}}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert!(compile("function f(o as Toybox.Lang.Object or Null) {return (o as Lang.ByteArray or Null).size();}").is_err());
}

#[test]
fn byte_array_tests_and_assertions_keep_api_three_requirements_in_saved_ir() {
    let modern = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(3, 0, 0)).unwrap();
    let old = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(2, 4, 0)).unwrap();
    for source in [
        "function f(o as Toybox.Lang.Object or Null) as Toybox.Lang.Boolean {return o instanceof Lang.ByteArray;}",
        "function f(o as Toybox.Lang.Object or Null) as Void {var unused=o as Lang.ByteArray;}",
        "function f(o as Toybox.Lang.Object or Null) as Toybox.Lang.ByteArray or Null {return o as Lang.ByteArray or Null;}",
    ] {
        let f = compile_with_api(source, modern).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert!(compile_with_api(source, old).is_err());
        assert!(parse_ir_with_api(&f.to_ir_text(), old).is_err());
    }
}
