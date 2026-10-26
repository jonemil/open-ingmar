#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{compile_sources, parse_unit_ir};

const PREFIX: &str = r#"
using Toybox.Lang;
typedef Pair as [Lang.Number,Lang.String];
typedef Record as interface {var pair as Pair;};
class Token {function value() as Lang.Number {return 41;}}
class Box {
    var pair as Pair;
    var optional as Pair or Null;
    function initialize(){self.pair=[1,"seed"];}
    function reset() as Void {self.pair=[2,"new"];}
    function value() as Lang.Number {self.reset();return 1;}
}
function noop() as Void {}
"#;

fn check(body: &str) -> ciq_compiler::CompilationUnit {
    let unit = compile_sources(&[PREFIX, body]).unwrap_or_else(|e| panic!("{body}\n{e:?}"));
    let text = unit.to_ir_text();
    assert_eq!(unit, parse_unit_ir(&text).unwrap(), "{text}");
    let asm = unit.lower();
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    unit
}

#[test]
fn removals_erase_only_the_addressed_view_and_preserve_boolean_results() {
    for body in [
        "function f(x as Pair) as Lang.Boolean {return x.remove(1);}",
        "function f(x as Pair) as Lang.Boolean {return x.removeAll(1);}",
        "function f(x as Pair) as Lang.Number {x.remove(1);return x.size();}",
        "function f(x as Pair) as Pair {x.remove(1);x=[2,\"new\"];return x;}",
        "function f(x as Pair) as Lang.Number {var changed=x.remove(1);if(changed){return x.size();}return 0;}",
        "function f(x as Pair or Null,n as Lang.Number) as Lang.Number {if(x!=null){for(var i=0;i<n;i++){x.removeAll(1);}return x.size();}return 0;}",
        "function f() as Lang.Number {var x=[1,\"s\"];var y=x;x.removeAll(1);y[0]=2;return x.size();}",
        "function f(x as Pair) as Lang.Number {var y=x;y.remove(1);return x[0];}",
        "function f(x as Pair,b as Lang.Boolean) as Lang.Number {if(b){x.remove(1);}return x.size();}",
        "function f(x as [Token,Lang.String],b as Lang.Boolean) as Lang.Number {if(b){x.removeAll(null);}return x.size();}",
        "function f(x as Pair,n as Lang.Number) as Lang.Number {for(var i=0;i<n;i++){x.remove(1);}return x.size();}",
        "function f(x as Pair,n as Lang.Number) as Lang.Number {while(n>0){x.removeAll(1);n--;}return x.size();}",
        "function f(x as Pair) as Lang.Number {do{x.remove(1);}while(false);return x.size();}",
        "function f(x as Pair or Null) as Lang.Number {if(x!=null){x.remove(1);return x.size();}return 0;}",
        "function f(x as Pair) as Lang.Number {x.remove(x.add(2).size());return x.size();}",
        "function f(x as Pair) as [Lang.Number,Lang.String,Lang.Boolean] {x.add(x.remove(1));return x;}",
        "function f(x as Pair) as [Lang.Boolean,Lang.String] {x[0]=x.remove(1);return x;}",
        "function f(x as Pair,b as Lang.Boolean) as Lang.Number {x.remove(b?1:2);return x.size();}",
        "function f(x as [Pair,Lang.Boolean]) as Lang.Number {x[0].remove(1);return x[0][0];}",
        "function f(x as Box) as Pair {x.pair.remove(1);return x.pair;}",
        "function f(x as Record) as Lang.Number {x.pair.remove(1);return x.pair.size();}",
        "class Holder {var pair as Pair;function initialize(){self.pair=[1,\"s\"];}
            function f() as Lang.Number {self.pair.remove(1);return self.pair.size();}}",
    ] {
        check(body);
    }
}

#[test]
fn stale_positional_and_typed_array_claims_are_rejected_after_removal() {
    for body in [
        "function f(x as Pair) as Pair {x.remove(1);return x;}",
        "function f(x as Pair) as Lang.Array<Lang.Number> {x.remove(1);return x;}",
        "function f(x as Pair,b as Lang.Boolean) as Pair {if(b){x.removeAll(1);}return x;}",
        "function f(x as Pair,n as Lang.Number) as Pair {while(n>0){x.remove(1);n--;}return x;}",
        "function f(x as Pair or Null) {x.remove(1);}",
        "function f(x as Record) as Pair {x.pair.removeAll(1);return x.pair;}",
        "function f(x as Pair) {x.remove();}",
        "function f(x as Pair) {x.removeAll(1,2);}",
    ] {
        assert!(
            checking::compile_sources(&[PREFIX, body]).is_err(),
            "{body}"
        );
    }
    let unit = check("function f(x as Pair) as Lang.Number {x.remove(1);return x.size();}");
    let text = unit.to_ir_text();
    assert!(text.contains("array-any"));
    let forged = text.replace("array-any", "tuple(integer;string)");
    assert_ne!(text, forged);
    assert!(parse_unit_ir(&forged).is_err(), "{forged}");
}

#[test]
fn immutable_receiver_captures_do_not_erase_a_rebound_storage_slot() {
    let text = r#"ciq-ir
.function f parameters=1 parameter-types=tuple(integer;string) result=integer
.parameter %0 0
.variable $0 %0
.read-variable %2 $0 tuple(integer;string)
.local $1 %2
.integer %4 7
.assign $0 %4
.read %6 $1
.integer %7 1
.call-native %8 %6 remove %7
.discard %8
.read-variable %10 $0 integer
.return %10
.end
"#;
    let function = ciq_compiler::parse_function_ir(text).unwrap();
    assert_eq!(
        function,
        ciq_compiler::parse_function_ir(&function.to_ir_text()).unwrap()
    );
    assert!(
        ciq_compiler::parse_function_ir(&text.replace("%10 $0 integer", "%10 $0 array-any"))
            .is_err()
    );
}

#[test]
fn removal_keeps_api_levels_and_callback_initialization_requirements() {
    let source = "function f(x as [Toybox.Lang.Number,Toybox.Lang.String]) as Toybox.Lang.Boolean {return x.remove(1);}";
    let context = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(5, 2, 0),
        ciq_compiler::api::ApiLevel(1, 2, 0),
    )
    .unwrap();
    assert!(ciq_compiler::compile_sources_with_api(&[source], context).is_err());
    let source = r#"using Toybox.Lang;using Toybox.System;
        class Token {function equals(value as Lang.Object or Null) as Lang.Boolean {System.exit();return false;}}
        function removing(x as [Token,Lang.String]) as Void {x.remove(null);}
        class Box {var value as Lang.String;function initialize(){removing([new Token(),"s"]);self.value="ready";}}
    "#;
    assert!(compile_sources(&[source]).is_err());
    check(
        "class Safe {var value as Lang.String;function initialize(){var x=[1,\"s\"];x.remove(1);var n=x.size();self.value=\"ready\";}}",
    );
}
