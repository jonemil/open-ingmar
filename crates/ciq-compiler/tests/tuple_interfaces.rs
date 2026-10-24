use ciq_compiler::{compile_sources, parse_unit_ir};

const TYPES: &str = r#"
using Toybox.Lang;
typedef Value as interface { var number as Lang.Number; function value() as Lang.Number; };
class Token {
    var number as Lang.Number;
    function initialize() { self.number=41; }
    function value() as Lang.Number {return self.number;}
}
class Other { function value() as Lang.String {return "wrong";} }
"#;

fn check(body: &str) -> ciq_compiler::CompilationUnit {
    let unit = compile_sources(&[TYPES, body]).unwrap_or_else(|e| panic!("{body}\n{e}"));
    let text = unit.to_ir_text();
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    unit
}

#[test]
fn source_tuple_witnesses_survive_all_storage_and_call_boundaries() {
    for body in [
        "function f() as [Value] {return [new Token()];}",
        "function f() as [Value] {var x=[new Token()];return x;}",
        "function f(x as Token or Null) as [Value or Null] {return [x];}",
        "function f(x as Token or Lang.Number) as [Value or Lang.Number] {return [x];}",
        "function f(x as Token or Other) as [Value or Other] {return [x];}",
        "function f() as Token or [Value] {return [new Token()];}",
        "function f() as [[Value]] {return [[new Token()]];}",
        "function f() as Lang.Array<[Value]> {return [[new Token()]];}",
        "function f() as [Lang.Array<Value>] {var x=[new Token()];return [x];}",
        "function f(x as [Token]) as [Value] {return x;}",
        "function f(x as [Token]) as Lang.Array<Value> {return x;}",
        "function take(x as [Value]) as Lang.Number {return x[0].value();} function f() as Lang.Number {return take([new Token()]);}",
        "class Box {var x as [Value];function initialize(){self.x=[new Token()];}function f() as [Value]{return self.x;}}",
        "function f(x as [Value]) as [Value] {x=[new Token()];return x;}",
        "function f(b as Lang.Boolean) as [Value] {return b?[new Token()]:[new Token()];}",
        "function f() as [interface {function value() as Lang.Number;}] {return [new Token()];}",
    ] {
        check(body);
    }
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-tuple-interfaces/app.mc"
    )])
    .unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn tuple_witnesses_reject_wrong_members_nullability_and_erased_provenance() {
    for body in [
        "function f() as [Value] {return [new Other()];}",
        "function f(x as Lang.Object) as [Value] {return [x];}",
        "function f(x as Token or Null) as [Value] {return [x];}",
        "function f(x as Token or Other) as [Value] {return [x];}",
        "function f() as [Value,Lang.String] {return [new Token(),1];}",
        "function f(x as Lang.Array<Token>) as Lang.Array<Value> {return x;}",
        "function f() as [interface {var number as Lang.Object;}] {return [new Token()];}",
        "function f() as [interface {function value(n as Lang.Number) as Lang.Number;}] {return [new Token()];}",
        "function take(x as Token) as Lang.Number {return x.value();} function f() as Lang.Number {return take([new Token()]);}",
    ] {
        assert!(compile_sources(&[TYPES, body]).is_err(), "accepted {body}");
    }
    let text = check("function f() as [Value] {return [new Token()];}").to_ir_text();
    let forged = text.replace(
        ".function value receiver=instance",
        ".function value receiver=instance parameters=1 parameter-types=integer",
    );
    assert_ne!(text, forged);
    assert!(parse_unit_ir(&forged).is_err());
}

#[test]
fn native_tuple_witnesses_and_named_interfaces_keep_api_requirements() {
    use ciq_compiler::{
        api::{ApiContext, ApiLevel},
        compile_function_with_api, compile_sources_with_api, parse_function_ir_with_api,
        parse_unit_ir_with_api,
    };
    let source = "typedef Convert as interface {function toDouble() as Toybox.Lang.Double or Null;}; function f() as [Convert] {return [\"1\"]; }";
    let available = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(3, 1, 0)).unwrap();
    let old = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(3, 0, 0)).unwrap();
    let f = compile_function_with_api(source, available).unwrap();
    assert!(compile_function_with_api(source, old).is_err());
    assert!(parse_function_ir_with_api(&f.to_ir_text(), old).is_err());
    let unit = compile_sources_with_api(&[source], available).unwrap();
    assert!(compile_sources_with_api(&[source], old).is_err());
    assert!(parse_unit_ir_with_api(&unit.to_ir_text(), old).is_err());
    let source = "class C {function compare(a as Toybox.Lang.Object,b as Toybox.Lang.Object) as Toybox.Lang.Number{return 0;}} function f() as [Toybox.Lang.Comparator] {return [new C()];}";
    let available = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap();
    let unit = compile_sources_with_api(&[source], available).unwrap();
    assert!(parse_unit_ir_with_api(&unit.to_ir_text(), old).is_err());
}
