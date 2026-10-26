#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{CompilationUnit, compile_sources, parse_unit_ir};

const DECLARATIONS: &str = r#"
using Toybox.Lang;
typedef Pair as [Lang.Number,Lang.String];
typedef Record as interface {var pair as Pair;};
typedef Other as interface {var pair as Pair;};
typedef Optional as interface {var pair as Pair or Null;};
class Box {var pair as Pair; function initialize(){self.pair=[1,"seed"];} }
function noop() as Void {}
"#;

fn check(body: &str) -> CompilationUnit {
    let unit = compile_sources(&[DECLARATIONS, body]).unwrap_or_else(|e| panic!("{body}\n{e:?}"));
    let text = unit.to_ir_text();
    assert_eq!(
        unit,
        parse_unit_ir(&text).unwrap_or_else(|e| panic!("{e:?}\n{text}")),
        "{text}"
    );
    let asm = unit.lower();
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    unit
}

#[test]
fn structural_tuple_fields_follow_declarations_and_not_owner_identity() {
    for body in [
        "function f(x as Record) as Lang.String {x.pair[0]=\"changed\";return x.pair[0];}",
        "function f(x as Record,y as Record) as Lang.String {x.pair[0]=\"changed\";return y.pair[0];}",
        "function f(x as Record,y as Other) as Lang.Number {x.pair[0]=\"changed\";return y.pair[0];}",
        "function f(x as Record or Other,y as Other) as Lang.Number {x.pair[0]=\"changed\";return y.pair[0];}",
        "function f(x as Record or Other,y as Other or Record) as Lang.String {x.pair[0]=\"changed\";return y.pair[0];}",
        "function f(x as Record) as Lang.String {var y=x;y.pair[0]=\"changed\";return x.pair[0];}",
        "function f(x as Record,y as Record) as Lang.String {x.pair[0]=\"changed\";x=y;return x.pair[0];}",
        "function f(x as Record) as [Lang.Number,Lang.String,Lang.Boolean] {x.pair.add(true);return x.pair;}",
        "function f(x as Record) as [Lang.Number,Lang.String,Lang.Number,Lang.String] {x.pair.addAll([2,\"tail\"]);return x.pair;}",
        "function f(x as Record) as Lang.Number {x.pair[0]=\"changed\";noop();return x.pair[0];}",
        "function f(x as Record) as Lang.Number {x.pair[0]=\"changed\";x.pair=[2,\"new\"];return x.pair[0];}",
        "function f(x as Record) as Lang.Number {var p=x.pair;p[0]=\"changed\";return x.pair[0];}",
        "function f(x as Record,b as Lang.Boolean) as Lang.Number or Lang.String {if(b){x.pair[0]=\"changed\";}return x.pair[0];}",
        "function f(x as Record) as Lang.Number {x.pair[0]+=2;x.pair[0]++;return x.pair[0];}",
        "function f(x as Record,n as Lang.Number) as Lang.Array<Lang.Number or Lang.String or Lang.Boolean> {for(var i=0;i<n;i++){x.pair.add(true);}return x.pair;}",
        "function f(x as interface {var pair as Pair;},y as interface {var pair as Pair;}) as Lang.Number {x.pair[0]=\"changed\";return y.pair[0];}",
    ] {
        check(body);
    }
}

#[test]
fn structural_field_views_recheck_null_guards_and_edited_ir() {
    for body in [
        "function f(x as Record) as Lang.Number {x.pair[0]=\"changed\";return x.pair[0];}",
        "function f(x as Record,y as Other) as Lang.String {x.pair[0]=\"changed\";return y.pair[0];}",
        "function f(x as Record) as Lang.String {x.pair[0]=\"changed\";noop();return x.pair[0];}",
        "function f(x as Record,b as Lang.Boolean) as Lang.Number {if(b){x.pair[0]=\"changed\";}return x.pair[0];}",
        "function f(x as Optional) {x.pair.add(true);}",
        "function f(x as Optional) {if(x.pair!=null){noop();x.pair.add(true);}}",
    ] {
        assert!(
            checking::compile_sources(&[DECLARATIONS, body]).is_err(),
            "{body}"
        );
    }
    for body in [
        "function f(x as Optional) as Lang.Number {if(x.pair!=null){x.pair.add(true);return x.pair.size();}return 0;}",
        "function f(x as Optional) as Lang.Number {x.pair=[1,\"new\"];x.pair.add(true);return x.pair.size();}",
        "function f(x as Optional) as Null {x.pair=null;return x.pair;}",
    ] {
        check(body);
    }
    let text =
        check("function f(x as Record) as Lang.String {x.pair[0]=\"changed\";return x.pair[0];}")
            .to_ir_text();
    assert!(text.contains("d(0@"));
    assert!(text.contains("view=tuple(string;string)"));
    for forged in [
        text.replace("view=tuple(string;string)", "view=tuple(integer;string)"),
        text.replace(" view=tuple(string;string)", ""),
        text.replace("view=tuple(string;string)", "view=void"),
        text.replace("d(0@", "d(-1@"),
    ] {
        assert!(parse_unit_ir(&forged).is_err(), "{forged}");
    }
}

#[test]
fn provenance_does_not_make_structural_assignment_or_signatures_nominal() {
    for body in [
        "function f(x as Record) as Other {return x;}",
        "function f(x as Record,y as Other) as Void {x=y;}",
        "function f(x as Lang.Array<Record>) as Lang.Array<Other> {return x;}",
        "function f(x as Lang.Array<Record or Other>) as Lang.Array<Record> {return x;}",
        "typedef Call as Method(x as Other) as Record; class Worker {function invoke(x as Record) as Other {return x;}} function f() as Call {var w=new Worker();return w.method(:invoke);}",
        "class Holder {var owner as Record;function initialize(x as Record){self.owner=x;}} typedef Parent as interface {var owner as Other;}; function f(x as Holder) as Parent {return x;}",
        "typedef A as interface {function use(x as Record) as Other;}; typedef B as interface {function use(x as Other) as Record;}; function f(x as A) as B {return x;}",
        "typedef A as interface {var child as Record;}; typedef B as interface {var child as Other;}; function f(x as A) as B {return x;}",
    ] {
        check(body);
    }
}

#[test]
fn inline_declarations_in_different_files_keep_distinct_saved_locations() {
    let a = "function first(x as interface {var pair as [Toybox.Lang.Number];}) {return x.pair;}";
    let b = "function other(x as interface {var pair as [Toybox.Lang.Number];}) {return x.pair;}";
    let unit = compile_sources(&[a, b]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains("d(0@"));
    assert!(text.contains("d(1@"));
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
}
