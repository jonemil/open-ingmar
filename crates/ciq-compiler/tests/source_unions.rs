use ciq_compiler::{compile_sources, parse_unit_ir};
const CLASSES: &str = "class A extends Toybox.Lang.Object{function initialize(){} function count() as Toybox.Lang.Number{return 7;}} class B extends Toybox.Lang.Object{function initialize(){} function size() as Toybox.Lang.Number{return 9;}}";

#[test]
fn exact_alternatives_survive_signatures_joins_and_both_guard_edges() {
    for body in [
        "function f(x as A or B) as Toybox.Lang.Number{if(x instanceof A){return x.count();}return x.size();}",
        "function f(x as B or A or B) as A or B{return x;}",
        "function f(x as A or B or Null) as Toybox.Lang.Number{if(x==null){return 0;}if(!(x instanceof A)){return x.size();}return x.count();}",
        "function f(b as Toybox.Lang.Boolean) as A or B{return b?new A():new B();}",
        "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.Number{var x=new A();if(b){x=new B();}if(x instanceof A){return x.count();}return x.size();}",
        "function f(x as A or B) as Toybox.Lang.Number{if(x instanceof A){x=new B();if(x instanceof B){return x.size();}}return 0;}",
        "module M{function echo(x as A or B) as A or B{return x;}}function f(b as Toybox.Lang.Boolean) as Toybox.Lang.Number{var x=M.echo(b?new A():new B());if(x instanceof A){return x.count();}return x.size();}",
        "class V extends Toybox.WatchUi.View{function initialize(){View.initialize();}function width() as Toybox.Lang.Number{return 3;}}function f(x as A or B or V) as Toybox.Lang.Number{if(x instanceof A){return x.count();}if(x instanceof B){return x.size();}return x.width();}",
    ] {
        let unit = compile_sources(&[CLASSES, body]).unwrap_or_else(|e| panic!("{body}: {e}"));
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
        assert_eq!(
            unit.lower(),
            ciq_assembly::Program::parse(&unit.lower().to_text()).unwrap()
        );
    }
}

#[test]
fn source_unions_reject_erasure_forgery_and_stale_proofs() {
    for body in [
        "function f(x as Toybox.Lang.Object) as A or B{return x;}",
        "function f(x as A or B) as A{return x;}",
        "function f(x as A or B){return x.count();}",
        "function f(x as A or B or Null){return x.count();}",
        "function f(x as A or B) as A or B{return null;}",
        "function f(x as A or Missing){return x;}",
        "function f(x as A or B){if(x instanceof A){x=new B();return x.count();}return 0;}",
        "module M{function echo(x as A or B){return x;}}function f(x as Toybox.Lang.Object){return M.echo(x);}",
    ] {
        assert!(compile_sources(&[CLASSES, body]).is_err(), "{body}");
    }
    let unit = compile_sources(&[CLASSES, "function f(x as A or B) as A or B{return x;}"]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains("source-union(source(A@Lang.Object)|source(B@Lang.Object))"));
    for changed in [
        text.replace("source(B@Lang.Object)", "source(Missing@Lang.Object)"),
        text.replace("source(B@Lang.Object)", "source(B@WatchUi.View)"),
        text.replace("source(B@Lang.Object)", "one-view"),
    ] {
        assert!(parse_unit_ir(&changed).is_err(), "{changed}");
    }
}

#[test]
fn interface_witnesses_cover_every_class_and_preserve_nullability() {
    let good = "class A{function initialize(){} function compare(a as Toybox.Lang.Object,b as Toybox.Lang.Object) as Toybox.Lang.Number{return 0;}} class B{function initialize(){} function compare(a as Toybox.Lang.Object,b as Toybox.Lang.Object) as Toybox.Lang.Number{return 0;}}";
    for body in [
        "function f(x as A or B) as Toybox.Lang.Comparator{return x;}",
        "function f(x as A or B or Null) as Toybox.Lang.Comparator or Null{return x;}",
        "function f(x as A or B){var a=[2,1];a.sort(x);return a[0];}",
    ] {
        let unit = compile_sources(&[good, body]).unwrap_or_else(|e| panic!("{body}: {e}"));
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
        let bad=good.replace("class B{function initialize(){} function compare(a as Toybox.Lang.Object,b as Toybox.Lang.Object) as Toybox.Lang.Number{return 0;}}","class B{function initialize(){}}");
        assert!(compile_sources(&[&bad, body]).is_err(), "{body}");
    }
    assert!(
        compile_sources(&[
            good,
            "function f(x as A or B or Null) as Toybox.Lang.Comparator{return x;}"
        ])
        .is_err()
    );
}

#[test]
fn typed_and_saved_singleton_sets_use_the_same_canonical_contract() {
    use ciq_compiler::{
        ClassBase, Function, ParameterType, ReturnType, SourceClass, SourceUnion, parse_function_ir,
    };
    let class = SourceClass {
        name: "A".into(),
        base: ClassBase::LangObject,
    };
    for nullable in [false, true] {
        let set = SourceUnion::new(vec![class.clone(), class.clone()], nullable).unwrap();
        assert_eq!(set.classes(), std::slice::from_ref(&class));
        let tag = if nullable {
            "nullable-source(A@Lang.Object)"
        } else {
            "source(A@Lang.Object)"
        };
        let text = format!(
            "ciq-ir\n.function f parameters=1 parameter-types={tag} result={tag}\n.parameter %0 0\n.return %0\n.end\n"
        );
        let parsed = parse_function_ir(&text).unwrap();
        let typed = Function::with_signature(
            "f".into(),
            vec![ParameterType::SourceUnion(set.clone())],
            ReturnType::SourceUnion(set),
            parsed.operations().to_vec(),
        )
        .unwrap();
        assert_eq!(typed, parsed);
        assert_eq!(typed, parse_function_ir(&typed.to_ir_text()).unwrap());
    }
    assert!(SourceUnion::new(vec![], true).is_err());
    assert!(
        SourceUnion::new(
            vec![SourceClass {
                name: "bad.name".into(),
                base: ClassBase::LangObject
            }],
            false
        )
        .is_err()
    );
}
