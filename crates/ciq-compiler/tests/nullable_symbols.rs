use ciq_compiler::{
    ParameterType, ReturnType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, compile_with_api, parse_ir, parse_unit_ir,
};

#[test]
fn nullable_symbols_keep_signatures_calls_joins_and_saved_stages() {
    let source = "module M {
        function choose(p as Toybox.Lang.Boolean) as Toybox.Lang.Symbol or Null {return p ? :alpha : null;}
        function reverse(p as Toybox.Lang.Boolean) as Toybox.Lang.Symbol or Null {return p ? null : :beta;}
        function forward(s as Toybox.Lang.Symbol or Null) as Toybox.Lang.Symbol or Null {return s;}
        function run() as Toybox.Lang.Symbol or Null {return M.forward(true ? M.choose(true) : :beta);}
    }";
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains("parameter-types=nullable-symbol result=nullable-symbol"));
    assert!(text.contains("type=nullable-symbol") || text.contains(" nullable-symbol"));
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    for source in [
        "function f() as Toybox.Lang.Symbol or Null {return :alpha;}",
        "function f() as Toybox.Lang.Symbol or Null {return null;}",
        "function f(s as Toybox.Lang.Symbol or Null) as Toybox.Lang.Symbol or Null {return s;}",
    ] {
        let function = compile(source).unwrap();
        assert_eq!(function.return_type(), ReturnType::NullableSymbol);
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    }
    let function = compile(
        "function f(s as Toybox.Lang.Symbol or Null) as Toybox.Lang.Symbol or Null {return s;}",
    )
    .unwrap();
    assert_eq!(function.parameter_types(), &[ParameterType::NullableSymbol]);
    let text = function.to_ir_text();
    for edited in [
        text.replace("result=nullable-symbol", "result=symbol"),
        text.replace("result=nullable-symbol", "result=nullable-integer"),
    ] {
        assert_ne!(text, edited);
        assert!(parse_ir(&edited).is_err());
    }
    let text = unit.to_ir_text();
    assert!(parse_unit_ir(&text.replace("result=nullable-symbol", "result=symbol")).is_err());
    let entry =
        compile_sources(&["function run() as Toybox.Lang.Symbol or Null {return null;}"]).unwrap();
    assert!(
        build_console_application(
            &entry,
            &ConsoleApplication {
                id: [0x18; 16],
                entry: "run".into()
            }
        )
        .is_err()
    );
}

#[test]
fn symbol_members_and_selectors_require_fresh_null_proofs() {
    for source in [
        "function f(s as Toybox.Lang.Symbol or Null) as Toybox.Lang.String {if(null==s){return \"missing\";} return s.toString();}",
        "function f(s as Toybox.Lang.Symbol or Null) as Toybox.Lang.Boolean {if(s!=null){return s==:alpha;}return false;}",
        "function f(s as Toybox.Lang.Symbol or Null) as Toybox.Lang.Boolean {if(null!=s){return 1 has s;}return false;}",
        "function f(s as Toybox.Lang.Symbol or Null) {if(s!=null){var m=1.method(s);}return 0;}",
        "function f(s as Toybox.Lang.Symbol or Null) {var local=s;while(local!=null){var n=local.toNumber();local=null;}return 0;}",
        "function f(s as Toybox.Lang.Symbol or Null) as Toybox.Lang.Symbol or Null {var local=s;local=:alpha;local=null;return local;}",
        "function f(s as Toybox.Lang.Symbol or Null) {return s!=null ? s.toNumber() : 0;}",
        "function f(s as Toybox.Lang.Symbol or Null) as Toybox.Lang.Boolean {return s!=null && s.equals(:alpha);}",
    ] {
        let function = compile(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    }
    for source in [
        "function f(s as Toybox.Lang.Symbol or Null) {return s.toNumber();}",
        "function f(s as Toybox.Lang.Symbol or Null) as Toybox.Lang.Boolean {return 1 has s;}",
        "function f(s as Toybox.Lang.Symbol or Null) {var m=1.method(s);return 0;}",
        "function f(s as Toybox.Lang.Symbol or Null) as Toybox.Lang.Symbol {return s;}",
        "function f(s as Toybox.Lang.Symbol or Null) {var local=s;if(local!=null){local=null;return local.toNumber();}return 0;}",
        "function f(s as Toybox.Lang.Symbol or Null) {var local=s;if(s!=null){return local.toNumber();}return 0;}",
        "function f(s as Toybox.Lang.Symbol or Null) {var present=s!=null;if(present){return s.toNumber();}return 0;}",
        "function f(s as Toybox.Lang.Symbol or Null) {var local=:alpha;local=s;return 0;}",
        "function f(s as Toybox.Lang.Symbol or Null) as Toybox.Lang.Boolean {return s==:alpha;}",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let source =
        "function f(s as Toybox.Lang.Symbol or Null) {if(s!=null){return s.toNumber();}return 0;}";
    let text = compile(source).unwrap().to_ir_text();
    let edited = text.replace(".not-equal ", ".equal ");
    assert_ne!(text, edited);
    assert!(parse_ir(&edited).is_err());
}

#[test]
fn nullable_storage_does_not_lower_symbol_method_api_requirements() {
    let initial = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap();
    let identity =
        "function f(s as Toybox.Lang.Symbol or Null) as Toybox.Lang.Symbol or Null {return s;}";
    assert!(compile_with_api(identity, initial).is_ok());
    let name = "function f(s as Toybox.Lang.Symbol or Null) as Toybox.Lang.String {if(s!=null){return s.toString();}return \"missing\";}";
    assert!(compile_with_api(name, initial).is_ok());
    let number =
        "function f(s as Toybox.Lang.Symbol or Null) {if(s!=null){return s.toNumber();}return 0;}";
    assert!(compile_with_api(number, initial).is_err());
    let modern = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(2, 3, 0)).unwrap();
    assert!(compile_with_api(number, modern).is_ok());
}

#[test]
fn nullable_symbol_assertions_preserve_bits_and_require_a_guard_for_use() {
    let source = "function f(s as Toybox.Lang.Object or Null) as Toybox.Lang.Symbol or Null {return s as Toybox.Lang.Symbol or Null;}";
    let function = compile(source).unwrap();
    let text = function.to_ir_text();
    assert!(text.contains(" nullable-symbol"));
    assert_eq!(function, parse_ir(&text).unwrap());
    let identity = compile(
        "function f(s as Toybox.Lang.Object or Null) as Toybox.Lang.Object or Null {return s;}",
    )
    .unwrap();
    assert_eq!(function.lower(), identity.lower());
    let guarded = "function f(s as Toybox.Lang.Object or Null) {var value=s as Toybox.Lang.Symbol or Null;if(value!=null){return value.toNumber();}return 0;}";
    assert!(compile(guarded).is_ok());
    let unsafe_use = "function f(s as Toybox.Lang.Object or Null) {var value=s as Toybox.Lang.Symbol or Null;return value.toNumber();}";
    assert!(compile(unsafe_use).is_err());
}
