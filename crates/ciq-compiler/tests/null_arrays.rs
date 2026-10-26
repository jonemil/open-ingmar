#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{ArrayElement, compile_sources, parse_unit_ir};

const PREFIX: &str = "using Toybox.Lang; typedef Nulls as Lang.Array<Null>;";

fn check(source: &str) -> ciq_compiler::CompilationUnit {
    let unit = compile_sources(&[PREFIX, source]).unwrap_or_else(|e| panic!("{source}\n{e:?}"));
    let text = unit.to_ir_text();
    assert_eq!(unit, parse_unit_ir(&text).unwrap(), "{text}");
    let asm = unit.lower();
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    unit
}

#[test]
fn null_only_arrays_preserve_their_contract_across_construction_and_methods() {
    for source in [
        "function f() as Nulls {return [null,null];}",
        "function f() as Nulls {return [];}",
        "function f(n as Lang.Number) as Nulls {return new[n];}",
        "function value() as Null {return null;} function f() as Nulls {return [value()];}",
        "function f(x as Nulls) as Null {x[0]=null;return x[0];}",
        "function f(x as Nulls,y as Nulls) as Nulls {return x.add(null).addAll(y);}",
        "function f(x as Nulls) as Nulls {return x.reverse().slice(null,1);}",
        "function f(x as Nulls) as Lang.Boolean {return x.remove(null);}",
        "function f(x as Nulls) as Lang.Boolean {return x.removeAll(null);}",
        "function f(x as Nulls) as Lang.Number {return x.indexOf(null);}",
        "function f(x as Nulls) as Lang.String {return x.toString();}",
        "function f(x as Nulls) as Lang.Boolean {x.remove(null);return x[0]==null;}",
        "function f(x as Nulls,y as Nulls,b as Lang.Boolean) as Nulls {return b?x:y;}",
        "function f(x as [Null],i as Lang.Number) as Nulls {x[i]=null;return x;}",
        "function f(x as [Null],tail as Nulls) as Nulls {x.addAll(tail);return x;}",
        "function f(x as [Null],n as Lang.Number) as Nulls {for(var i=0;i<n;i++){x.add(null);}return x;}",
        "class Box {var values as Nulls;function initialize(){self.values=[null];} function read() as Null{return self.values[0];}}",
    ] {
        check(source);
    }
    let f = ciq_compiler::compile_function(
        "function f() as Toybox.Lang.Array<Null>{return [null,null];}",
    )
    .unwrap();
    assert_eq!(
        f.return_type(),
        ciq_compiler::ReturnType::Array(ArrayElement::Null)
    );
    assert!(f.to_ir_text().contains("null-array"));
}

#[test]
fn null_only_arrays_reject_non_null_writes_and_invariant_widening() {
    for source in [
        "function f(x as Nulls) {x[0]=1;}",
        "function f(x as Nulls) {x.add(1);}",
        "function f(x as Nulls,y as Lang.Array<Lang.Number>) {x.addAll(y);}",
        "function f(x as Nulls) {x.indexOf(1);}",
        "function f(x as Nulls) {x.remove(1);}",
        "function f(x as Nulls) {x.removeAll(1);}",
        "function f(x as Nulls) as Lang.Number {return x[0];}",
        "function f(x as Nulls) as Lang.Array<Lang.Number or Null> {return x;}",
        "function f(x as Lang.Array<Lang.Number or Null>) as Nulls {return x;}",
        "function f() as Nulls {return [null,1];}",
    ] {
        assert!(
            checking::compile_sources(&[PREFIX, source]).is_err(),
            "{source}"
        );
    }
    let text =
        check("function f() as Lang.Array<Lang.Number or Null>{return [1,null];}").to_ir_text();
    let forged = text.replace("nullable-number", "null");
    assert_ne!(text, forged);
    assert!(parse_unit_ir(&forged).is_err(), "{forged}");
}
