use ciq_compiler::{
    ParameterType, ReturnType,
    api::{ApiContext, ApiLevel},
    compile, compile_sources, compile_with_api, parse_ir, parse_ir_with_api, parse_unit_ir,
};

#[test]
fn nullable_methods_preserve_signatures_calls_and_saved_stages() {
    let unit = compile_sources(&[r#"
module M {
    function choose(m as Toybox.Lang.Method, b as Toybox.Lang.Boolean) as Toybox.Lang.Method or Null { return b ? m : null; }
    function reverse(m as Toybox.Lang.Method, b as Toybox.Lang.Boolean) as Toybox.Lang.Method or Null { return b ? null : m; }
    function forward(m as Toybox.Lang.Method or Null) as Toybox.Lang.Method or Null { return m; }
    function run(m as Toybox.Lang.Method) as Toybox.Lang.Method or Null { return M.forward(M.choose(m, true)); }
}
"#]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains("parameter-types=nullable-method result=nullable-method"));
    assert!(text.contains(" nullable-method"));
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    assert!(parse_unit_ir(&text.replace("result=nullable-method", "result=method")).is_err());
    for source in [
        "function f(m as Toybox.Lang.Method) as Toybox.Lang.Method or Null { return m; }",
        "function f() as Toybox.Lang.Method or Null { return null; }",
        "function f(m as Toybox.Lang.Method or Null) as Toybox.Lang.Method or Null { return m; }",
    ] {
        let function = compile(source).unwrap();
        assert_eq!(function.return_type(), ReturnType::NullableMethod);
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    }
    let f = compile(
        "function f(m as Toybox.Lang.Method or Null) as Toybox.Lang.Method or Null {return m;}",
    )
    .unwrap();
    assert_eq!(f.parameter_types(), &[ParameterType::NullableMethod]);
    assert!(
        parse_ir(
            &f.to_ir_text()
                .replace("result=nullable-method", "result=method")
        )
        .is_err()
    );
    assert!(
        parse_ir(
            &f.to_ir_text()
                .replace("result=nullable-method", "result=nullable-symbol")
        )
        .is_err()
    );
}

#[test]
fn nullable_method_members_need_fresh_storage_proofs() {
    for source in [
        "function f(m as Toybox.Lang.Method or Null) as Void {if(m!=null){m.invoke();}}",
        "function f(m as Toybox.Lang.Method or Null) as Toybox.Lang.Object or Null {if(null==m){return null;} return m.invoke();}",
        "function f(m as Toybox.Lang.Method or Null) as Toybox.Lang.Boolean {return m!=null && m.equals(m);}",
        "function f(m as Toybox.Lang.Method or Null) as Toybox.Lang.String {return m!=null ? m.toString() : \"missing\";}",
        "function f(m as Toybox.Lang.Method or Null) as Void {var local=m; while(local!=null){local.invoke();local=null;}}",
        "function f(m as Toybox.Lang.Method or Null, n as Toybox.Lang.Method) as Toybox.Lang.Method or Null {var local=m;local=n;local=null;return local;}",
        "function f(m as Toybox.Lang.Method or Null) as Toybox.Lang.Boolean {if(m!=null){return m!=null;}return m==null;}",
    ] {
        let f = compile(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    for source in [
        "function f(m as Toybox.Lang.Method or Null) as Void {m.invoke();}",
        "function f(m as Toybox.Lang.Method or Null) as Toybox.Lang.String {return m.toString();}",
        "function f(m as Toybox.Lang.Method or Null) as Toybox.Lang.Method {return m;}",
        "function f(m as Toybox.Lang.Method or Null) as Void {var local=m;if(local!=null){local=null;local.invoke();}}",
        "function f(m as Toybox.Lang.Method or Null) as Void {var local=m;if(m!=null){local.invoke();}}",
        "function f(m as Toybox.Lang.Method or Null) as Void {var present=m!=null;if(present){m.invoke();}}",
        "function f(m as Toybox.Lang.Method or Null,n as Toybox.Lang.Method) as Void {var local=n;local=m;}",
        "function f(m as Toybox.Lang.Method or Null,n as Toybox.Lang.Method) as Toybox.Lang.Boolean {return m==n;}",
        "function f(m as Toybox.Lang.Method or Null) as Void {if(m){m.invoke();}}",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    let f =
        compile("function f(m as Toybox.Lang.Method or Null) as Void {if(m!=null){m.invoke();}}")
            .unwrap();
    let text = f.to_ir_text();
    let edited = text.replace(".not-equal ", ".equal ");
    assert_ne!(text, edited);
    assert!(parse_ir(&edited).is_err());
}

#[test]
fn nullable_method_assertions_keep_runtime_values_and_api_guards() {
    let source = "function f(m as Toybox.Lang.Object or Null) as Toybox.Lang.Method or Null {return m as Toybox.Lang.Method or Null;}";
    let f = compile(source).unwrap();
    let identity = compile(
        "function f(m as Toybox.Lang.Object or Null) as Toybox.Lang.Object or Null {return m;}",
    )
    .unwrap();
    assert_eq!(f.lower(), identity.lower());
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let initial = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap();
    let member =
        "function f(m as Toybox.Lang.Method or Null) {if(m!=null){return m.hashCode();}return 0;}";
    assert!(compile_with_api(member, initial).is_ok());
    let weak = "function f(m as Toybox.Lang.Method or Null) as Void {if(m!=null){m.weak();}}";
    let modern = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 2, 0)).unwrap();
    let f = compile_with_api(weak, modern).unwrap();
    assert!(compile_with_api(weak, initial).is_err());
    assert!(parse_ir_with_api(&f.to_ir_text(), initial).is_err());
}
