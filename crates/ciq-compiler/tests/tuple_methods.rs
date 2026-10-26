#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    compile_sources, parse_unit_ir,
};

fn check(body: &str) -> ciq_compiler::CompilationUnit {
    let prefix = r#"using Toybox.Lang;
        typedef Pair as [Lang.Number,Lang.String];
        class Token {function number() as Lang.Number {return 41;}}
        class Order {function compare(a as Lang.Object,b as Lang.Object) as Lang.Number {return 0;}}
    "#;
    let unit = compile_sources(&[prefix, body]).unwrap_or_else(|e| panic!("{body}\n{e:?}"));
    let text = unit.to_ir_text();
    assert_eq!(unit, parse_unit_ir(&text).unwrap(), "{text}");
    let asm = unit.lower();
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    unit
}

#[test]
fn copies_search_sort_and_object_methods_keep_saved_contracts() {
    for body in [
        "function f(x as Pair) as [Lang.String,Lang.Number] {return x.reverse();}",
        "function f(x as [Token,[Lang.String]]) as Lang.Number {return x.reverse()[1].number();}",
        "function f(x as Pair) as [Lang.Number] {return x.slice(0,1);}",
        "function f(x as Pair) as Pair {return x.slice(null,null);}",
        "function f(x as Pair) as [Lang.String] {return x.slice(-1,null);}",
        "function f(x as Pair) as Pair {return x.slice(-99,99);}",
        "function f(x as Pair) as Lang.Number {return x.slice(2,0).size();}",
        "function f(x as [Token,Lang.String]) as Lang.Number {return x.slice(0,1)[0].number();}",
        "function f(x as Pair,i as Lang.Number) as Lang.Number {return x.slice(i,null).size();}",
        "function f(x as Pair) as Lang.Number {var i=0;return x.slice(i,null).size();}",
        "function f(x as Pair) as Lang.Number {return x.indexOf(true);}",
        "function f(x as [Token,Lang.String],token as Token) as Lang.Number {return x.indexOf(token);}",
        "function f(x as [Lang.Number,Lang.Number]) as Void {x.sort(null);}",
        "function f(x as Pair) as Void {x.sort(null);}",
        "function f(x as [[Lang.Number],Lang.String],order as Lang.Comparator) as Void {x.sort(order);}",
        "function f(x as Pair) as Lang.WeakReference {return x.weak();}",
        "function f(x as Pair) as Method() as Lang.Number {return x.method(:size);}",
        "function f(x as Pair) as Method() as Lang.Array<Lang.Object or Null> {return x.method(:reverse);}",
        "function f(x as Pair) as Method(a as Lang.Number or Null,b as Lang.Number or Null) as Lang.Array<Lang.Object or Null> {return x.method(:slice);}",
    ] {
        check(body);
    }
}

#[test]
fn native_profiles_and_literal_slice_proofs_are_rechecked() {
    for body in [
        "function f(x as Pair) as Pair {return x.reverse();}",
        "function f(x as Pair,i as Lang.Number) as [Lang.Number] {return x.slice(i,1);}",
        "function f(x as Pair,i as Lang.Number) as Lang.Array<Lang.Number> {return x.slice(i,1);}",
        "function f(x as Pair) as [Lang.Number] {var i=0;return x.slice(i,1);}",
        "function f(x as Pair) {return x.slice(true,1);}",
        "function f(x as Pair) {return x.slice(0);}",
        "function f(x as Pair) {return x.indexOf();}",
        "function f(x as Pair) {return x.sort(null);}",
        "function f(x as Pair) {x.sort(1);}",
        "function f(x as Pair or Null) {return x.reverse();}",
    ] {
        assert!(
            checking::compile_sources(&[&format!(
                "using Toybox.Lang;typedef Pair as [Lang.Number,Lang.String];{body}"
            )])
            .is_err(),
            "{body}"
        );
    }
    let text = check("function f(x as Pair) as [Lang.Number] {return x.slice(0,1);}").to_ir_text();
    let changed = text.replace(" 1 span=", " 2 span=");
    assert_ne!(text, changed);
    assert!(parse_unit_ir(&changed).is_err());
    let source =
        "function f(x as [Toybox.Lang.Number]) as Toybox.Lang.Number {return x.indexOf(1);}";
    let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 2, 0)).unwrap();
    assert!(ciq_compiler::compile_sources_with_api(&[source], context).is_err());
}

#[test]
fn tuple_sort_callbacks_still_participate_in_initialization_effect_checks() {
    let source = r#"using Toybox.Lang;using Toybox.System;
        class Order {function compare(a as Lang.Object,b as Lang.Object) as Lang.Number {System.exit();return 0;}}
        function sorting(x as [Lang.Number,Lang.Number]) as Void {x.sort(new Order());}
        class Box {var value as Lang.String;function initialize(){sorting([2,1]);self.value="ready";}}
    "#;
    assert!(compile_sources(&[source]).is_err());
}
