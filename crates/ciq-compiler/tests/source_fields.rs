use ciq_compiler::{CompilationUnit, compile_sources, parse_unit_ir};

const CLASSES: &str = r#"
using Toybox.Lang;
typedef Pair as [Lang.Number,Lang.String];
class Token { function value() as Lang.Number {return 41;} }
class Box {
    var count = 1;
    var pair as Pair;
    var token as Token;
    var optional as Token or Null;
    function initialize() {self.pair=[1,"seed"];self.token=new Token();}
}
class Other {var count=2;}
class Wrong {var count as Lang.String;function initialize(){self.count="wrong";}}
"#;

fn check(body: &str) -> CompilationUnit {
    let unit =
        compile_sources(&[CLASSES, body]).unwrap_or_else(|error| panic!("{body}\n{error:?}"));
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let asm = unit.lower();
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    unit
}

#[test]
fn source_instance_fields_share_declared_contracts_across_saved_stages() {
    for body in [
        "function f(x as Box) as Lang.Number {return x.count;}",
        "function f(x as Box) as Lang.Number {x.count=4;x.count+=2;x.count++;x.count--;return x.count;}",
        "function f(x as Box,b as Lang.Boolean) as Lang.Number {x.count=b?5:6;return x.count;}",
        "function f(x as Box) as Lang.Number {return x.token.value();}",
        "function f(x as Box) as Lang.Number {x.token=new Token();return x.token.value();}",
        "function f(x as Box) as Lang.Number {var t=x.optional;if(t!=null){return t.value();}return 0;}",
        "function f(x as Box) as Void {x.optional=new Token();x.optional=null;}",
        "function f(x as Box) as Lang.String {x.pair=[2,\"new\"];return x.pair[1];}",
        "function f(x as Box) as Lang.Number {x.pair[0]=5;x.pair[0]+=2;x.pair[0]++;return x.pair[0];}",
        "function f(x as Box) as [Lang.Number,Lang.String,Lang.Boolean] {return x.pair.add(true);}",
        "function f(x as Box) as Lang.Number {x.pair[0]=\"changed\";return x.pair[0];}",
        "function f(x as Box) as Pair {x.pair.add(true);return x.pair;}",
        "function f(x as Box or Other) as Lang.Number {x.count=7;return x.count;}",
        "function f(x as Box or Null) as Lang.Number {if(x!=null){return x.count;}return 0;}",
        "function f(x as Box,b as Lang.Boolean) as Lang.Number {var y=x;while(b){y=new Box();b=false;}return y.count;}",
    ] {
        check(body);
    }
}

#[test]
fn receiver_contracts_and_saved_field_metadata_are_independently_checked() {
    for body in [
        "function f(x as Box or Null) as Lang.Number {return x.count;}",
        "function f(x as Box) {x.count=\"wrong\";}",
        "function f(x as Box) {return x.missing;}",
        "function f(x as Box) {x.missing=1;}",
        "function f(x as Box) {x.token=null;}",
        "function f(x as Box) {x.token=new Other();}",
        "function f(x as Box) as Lang.String {x.pair[0]=\"changed\";return x.pair[0];}",
        "function f(x as Box or Wrong) {return x.count;}",
        "function f(x as Lang.Object) {return x.count;}",
    ] {
        assert!(compile_sources(&[CLASSES, body]).is_err(), "{body}");
    }
    let unit = check("function f(x as Box) as Lang.Number {x.count=2;return x.count;}");
    let text = unit.to_ir_text();
    assert!(text.contains("field=integer"));
    for forged in [
        text.replace("field=integer", "field=long"),
        text.replace("count field=integer", "missing field=integer"),
        text.replace(" field=integer", ""),
        text.replace("field=integer", "field=void"),
        text.replace("field=integer", "field=integer trailing"),
    ] {
        assert!(parse_unit_ir(&forged).is_err(), "{forged}");
    }
    // Any results do not constrain the claimed read type. Only the declaring
    // unit can reject these otherwise well-typed fabricated field contracts.
    let text = check("function f(x as Box) {return x.count;}").to_ir_text();
    for tag in ["string", "source(Token@Lang.Object)"] {
        let forged = text.replace("field=integer", &format!("field={tag}"));
        let error = parse_unit_ir(&forged).unwrap_err();
        assert!(
            error.to_string().contains("does not match its declaration"),
            "{error}"
        );
    }
}
