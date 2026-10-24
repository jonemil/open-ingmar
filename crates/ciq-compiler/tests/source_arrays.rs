use ciq_compiler::{compile_sources, parse_unit_ir};
const PREFIX: &str = r#"
using Toybox.Lang;
class A {function value() as Lang.Number {return 41;}}
class B {function value() as Lang.Number {return 42;}}
typedef As as Lang.Array<A>;
typedef MaybeAs as Lang.Array<A or Null>;
"#;
fn check(source: &str) -> ciq_compiler::CompilationUnit {
    let unit = compile_sources(&[PREFIX, source]).unwrap_or_else(|e| panic!("{source}\n{e:?}"));
    let text = unit.to_ir_text();
    assert_eq!(unit, parse_unit_ir(&text).unwrap(), "{text}");
    let asm = unit.lower();
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    unit
}
#[test]
fn source_elements_survive_construction_native_methods_and_shape_loss() {
    for source in [
        "function f() as As {return [new A(),new A()];}",
        "function f() as As {return [];}",
        "function f() as MaybeAs {return [new A(),null];}",
        "function f() as MaybeAs {return [null];}",
        "function f(n as Lang.Number) as MaybeAs {return new[n];}",
        "function f(x as As) as Lang.Number {return x[0].value();}",
        "function f(x as As) {x[0]=new A();x.add(new A()).addAll([new A()]);}",
        "function f(x as As) as A {return x.reverse().slice(null,1)[0];}",
        "function f(x as As,a as A) as Lang.Boolean {return x.remove(a);}",
        "function f(x as As,a as A) as Lang.Boolean {return x.removeAll(a);}",
        "function f(x as As,a as A) as Lang.Number {return x.indexOf(a);}",
        "function f(x as MaybeAs) as Lang.Number {var a=x[0];if(a!=null){return a.value();}return 0;}",
        "function f(x as Lang.Array<A or B>) as Lang.Number {return x[0].value();}",
        "function f(x as [A],tail as As) as As {x.addAll(tail);return x;}",
        "function f(x as [A],i as Lang.Number) as As {x[i]=new A();return x;}",
        "function f(x as [A],n as Lang.Number) as As {for(var i=0;i<n;i++){x.add(new A());}return x;}",
        "function f() as Lang.Array<As> {return [[new A()]];}",
        "function f(x as [A,Lang.String],tail as Lang.Array<Lang.Number>) {x.addAll(tail);}",
        "function f(x as [A,Lang.String],i as Lang.Number) as Lang.Array<A or Lang.String> {x[i]=new A();return x;}",
        "function f(x as As) as Lang.Method {return x[0].method(:value);}",
        "function f(x as Lang.Array<A or B>) as Lang.Number {var v=x[0];if(v instanceof A){return v.value();}return 0;}",
        "class Box {var values as As;function initialize(){self.values=[new A()];}function f() as A {return self.values[0];}}",
    ] {
        check(source);
    }
}
#[test]
fn native_bases_and_unrelated_source_classes_cannot_forge_element_identity() {
    for source in [
        "function f(x as As) {x[0]=new B();}",
        "function f(x as As) {x.add(new B());}",
        "function f(x as As,a as Lang.Object) {x.add(a);}",
        "function f(x as As,a as B) {x.remove(a);}",
        "function f(x as As,a as B) {x.indexOf(a);}",
        "function f(x as As) {x.add(null);}",
        "function f(x as As) as B {return x[0];}",
        "function f(x as As) as Lang.Array<Lang.Object> {return x;}",
        "function f(x as Lang.Array<Lang.Object>) as As {return x;}",
        "function f() as As {return [new B()];}",
        "function f() as As {return new[2];}",
        "function f(xs as Lang.Array<A or Toybox.Graphics.Dc>) {}",
    ] {
        assert!(compile_sources(&[PREFIX, source]).is_err(), "{source}");
    }
    let text = check("function f(x as As) as Lang.Number {return x[0].value();}").to_ir_text();
    let forged = text.replace(
        "exact(source(A@Lang.Object))",
        "exact(source(B@Lang.Object))",
    );
    assert_ne!(text, forged);
    assert!(parse_unit_ir(&forged).is_err(), "{forged}");
}

#[test]
fn source_equality_and_observers_keep_constructor_callback_checks() {
    for effect in [
        "values.toString();",
        "values.indexOf(a);",
        "values.remove(a);",
        "values.removeAll(a);",
    ] {
        let source = format!(
            "class Box {{var a as A;function initialize(values as As,a as A) {{{effect}self.a=a;}}}}"
        );
        let err = compile_sources(&[PREFIX, &source]).unwrap_err().to_string();
        assert!(err.contains("shutdown"), "{err}");
        let safe = source.replace(&format!("{effect}self.a=a;"), &format!("self.a=a;{effect}"));
        check(&safe);
    }
}
#[test]
fn sorting_uses_a_checked_comparator_and_retains_source_elements() {
    check(
        "class Order {function compare(a as Lang.Object,b as Lang.Object) as Lang.Number {var x=a as A;var y=b as A;return x.value()-y.value();}} function f(values as As) as A {values.sort(new Order());return values[0];}",
    );
    assert!(compile_sources(&[PREFIX, "function f(values as As) {values.sort(null);}"]).is_err());
}

#[test]
fn saved_source_element_declarations_must_resolve_even_without_a_read() {
    let text = check("function f(x as As) as Lang.Number {return 0;}").to_ir_text();
    let forged = text.replace(
        "exact(source(A@Lang.Object))",
        "exact(source(Missing@Lang.Object))",
    );
    assert_ne!(text, forged);
    assert!(parse_unit_ir(&forged).is_err(), "{forged}");
}
