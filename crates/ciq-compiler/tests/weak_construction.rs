use ciq_compiler::{
    Operation,
    api::{ApiContext, ApiLevel},
    compile_function_with_api, compile_sources, parse_function_ir, parse_function_ir_with_api,
    parse_unit_ir,
};

#[test]
fn zero_argument_weak_construction_keeps_its_distinct_operation_and_api_guard() {
    let api = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 2, 0)).unwrap();
    let old = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 1, 0)).unwrap();
    let source =
        "function f() as Toybox.Lang.WeakReference {return new Toybox.Lang.WeakReference();}";
    let f = compile_function_with_api(source, api).unwrap();
    assert!(matches!(
        f.operations()[0].operation,
        Operation::WeakReferenceNew { .. }
    ));
    let text = f.to_ir_text();
    assert!(text.contains(".new-native %0 Toybox.Lang.WeakReference"));
    assert_eq!(f, parse_function_ir(&text).unwrap());
    assert!(compile_function_with_api(source, old).is_err());
    assert!(parse_function_ir_with_api(&text, old).is_err());
    let assembly = f.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    let asm = assembly.to_text();
    assert!(asm.contains("__ciq_sdk_Lang"));
    assert!(asm.contains("__ciq_type_WeakReference"));
    assert!(asm.contains("newc"));
    assert!(!asm.contains("__ciq_native_weak"));
}

#[test]
fn construction_uses_aliases_and_imports_and_rejects_arguments_and_shadowing() {
    for source in [
        "using Toybox.Lang as L;function f() as L.WeakReference {return new L.WeakReference();}",
        "import Toybox.Lang;function f() as WeakReference {return new WeakReference();}",
        "module M {function make() as Toybox.Lang.WeakReference{return new Toybox.Lang.WeakReference();}function run() as Toybox.Lang.WeakReference{return M.make();}}",
    ] {
        let unit = compile_sources(&[source]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
    for source in [
        "function f() as Toybox.Lang.WeakReference {return new Toybox.Lang.WeakReference(1);}",
        "using Toybox.Lang as L;function f(L) as Toybox.Lang.WeakReference {return new L.WeakReference();}",
        "import Toybox.Lang;function f(WeakReference) as Toybox.Lang.WeakReference {return new WeakReference();}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "accepted {source}");
    }
    let text = "ciq-ir\n.function f result=weak-reference\n.new-native %0 Toybox.Lang.WeakReference extra\n.return %0\n.end\n";
    assert!(parse_function_ir(text).is_err());
}
