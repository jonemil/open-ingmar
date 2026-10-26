#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{compile_sources, parse_unit_ir};

fn check(source: &str) -> ciq_compiler::CompilationUnit {
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert_eq!(unit, parse_unit_ir(&text).unwrap(), "{text}");
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    unit
}

const PREFIX: &str = r#"
using Toybox.Lang;
typedef Pair as [Lang.Number, Lang.String];
class Box {
    var pair as Pair;
    var optional as Pair or Null;
    function initialize() { self.pair=[1,"seed"]; }
    function reset() as Void { self.pair=[7,"reset"]; }
    function noop() as Void {}
    function resetAndValue() as Lang.Boolean { self.reset(); return true; }
"#;

#[test]
fn field_mutation_reads_branches_calls_and_reassignment_resume_from_ir() {
    for body in [
        r#"function f() as Lang.String {self.pair[0]="changed"; return self.pair[0];}"#,
        r#"function f() as [Lang.Number,Lang.String,Lang.Boolean] {self.pair.add(true); return self.pair;}"#,
        r#"function f() as [Lang.Number,Lang.String,Lang.Boolean] {return self.pair.add(true);}"#,
        r#"function f() as [Lang.Number,Lang.String,Lang.Number,Lang.String] {self.pair.addAll([2,"tail"]); return self.pair;}"#,
        r#"function f(b as Lang.Boolean) as Lang.Number or Lang.Boolean {if(b){self.pair[0]=true;}return self.pair[0];}"#,
        r#"function f(b as Lang.Boolean) as Lang.Number or Lang.String {if(b){self.pair[0]="changed";}else{self.pair=[2,"other"];}return self.pair[0];}"#,
        r#"function f() as Lang.Number {self.pair[0]="changed";self.reset();return self.pair[0];}"#,
        r#"function f() as Lang.Number {self.pair[0]="changed";self.noop();return self.pair[0];}"#,
        r#"function f() as Lang.Number {self.pair[0]="changed";self.pair=[7,"new"];return self.pair[0];}"#,
        r#"function f() as Lang.Number {var old=self.pair;old[0]="changed";return self.pair[0];}"#,
        r#"function f(n as Lang.Number) as Lang.Array<Lang.Number or Lang.String or Lang.Boolean> {for(var i=0;i<n;i++){self.pair.add(true);}return self.pair;}"#,
        r#"function f(i as Lang.Number) as Lang.Array<Lang.Number or Lang.String or Lang.Boolean> {self.pair[i]=false;return self.pair;}"#,
        r#"function f() as Lang.Number {self.pair[0]+=2;self.pair[0]++;return self.pair[0];}"#,
        r#"function f() as Lang.Number {self.pair.add(self.resetAndValue());return self.pair.size();}"#,
        r#"function f(value as Lang.Object) as Lang.String {if(value instanceof Lang.String){self.pair[0]=value;return self.pair[0];}return "other";}"#,
    ] {
        check(&format!("{PREFIX}{body}}}"));
    }
}

#[test]
fn nullable_field_guards_and_initialization_keep_real_field_reads() {
    for body in [
        r#"function f() as Lang.Number {if(self.optional!=null){self.optional.add(true);return self.optional.size();}return 0;}"#,
        r#"function f() as Null {self.optional=null;return self.optional;}"#,
        r#"function f() as Lang.String {self.optional=[7,"fresh"];self.optional[0]="changed";return self.optional[0];}"#,
        r#"function f(n as Lang.Number) as Lang.Number {if(self.optional!=null){for(var i=0;i<n;i++){self.optional.add(true);}return self.optional.size();}return 0;}"#,
    ] {
        check(&format!("{PREFIX}{body}}}"));
    }
    check(
        r#"class Box { var pair as [Toybox.Lang.Number,Toybox.Lang.String]; function initialize() {self.pair=[1,"seed"];self.pair[0]=2;} }"#,
    );
    check(
        r#"class Token {} class Mixed {
        var item as Token or [Toybox.Lang.Number] or Null;
        function assign() as Token { self.item=new Token(); return self.item; }
        function guarded() as Token or [Toybox.Lang.Number] { if(self.item!=null) {return self.item;} return [0]; }
    }"#,
    );
}

#[test]
fn stale_field_reads_and_forged_declarations_fail_validation() {
    for body in [
        r#"function f() as Lang.Number {self.pair[0]="changed";return self.pair[0];}"#,
        r#"function f(b as Lang.Boolean) as Lang.Number {if(b){self.pair[0]=true;}return self.pair[0];}"#,
        r#"function f() as Lang.String {self.pair[0]="changed";self.noop();return self.pair[0];}"#,
        r#"function f() as Lang.Number {self.optional.add(true);return 0;}"#,
        r#"function f() as Lang.Number {if(self.optional!=null){self.noop();self.optional.add(true);}return 0;}"#,
    ] {
        let source = format!("{PREFIX}{body}}}");
        assert!(checking::compile_sources(&[&source]).is_err(), "{source}");
    }
    let unit = check(&format!(
        "{PREFIX}function f() as Lang.String {{self.pair[0]=\"changed\";return self.pair[0];}}}}"
    ));
    let text = unit.to_ir_text();
    assert!(text.contains("declared=tuple(integer;string)"));
    let forged = text.replace(
        "result=tuple(string;string)",
        "result=tuple(integer;string)",
    );
    assert_ne!(text, forged);
    assert!(parse_unit_ir(&forged).is_err());
    assert!(
        parse_unit_ir(&text.replace(
            "declared=tuple(integer;string)",
            "declared=tuple(boolean;string)"
        ))
        .is_err()
    );
    assert!(parse_unit_ir(&text.replace(" declared=tuple(integer;string)", "")).is_err());
}
