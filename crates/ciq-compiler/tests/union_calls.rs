use ciq_compiler::{compile_sources, parse_unit_ir};
const CLASSES: &str = "class A{function initialize(){} function value(x as Toybox.Lang.Number) as Toybox.Lang.Number{return x+1;}function accept(x as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number{return x.size();}} class B{function initialize(){} function value(x as Toybox.Lang.Number) as Toybox.Lang.String{return x.toString();}function accept(x as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number{return x.size();}}";
#[test]
fn common_calls_join_results_and_preserve_text_and_assembly() {
    for body in [
        "function f(x as A or B) as Toybox.Lang.Number or Toybox.Lang.String{return x.value(2);}",
        "function f(x as A or B){x.value(2);return x.accept([]);}",
        "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.Number or Toybox.Lang.String{return (b?new A():new B()).value(2);}",
        "module M{function make(b as Toybox.Lang.Boolean) as A or B{return b?new A():new B();}}function f(){return M.make(true).value(2);}",
    ] {
        let unit = compile_sources(&[CLASSES, body]).unwrap_or_else(|e| panic!("{body}: {e}"));
        assert!(unit.to_ir_text().contains(" A|B "));
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
        assert_eq!(
            unit.lower(),
            ciq_assembly::Program::parse(&unit.lower().to_text()).unwrap()
        );
    }
}
#[test]
fn every_signature_and_receiver_alternative_must_be_valid() {
    for classes in [
        CLASSES.replace(
            "function value(x as Toybox.Lang.Number) as Toybox.Lang.String{return x.toString();}",
            "",
        ),
        CLASSES.replace(
            "function value(x as Toybox.Lang.Number) as Toybox.Lang.String",
            "function value(x as Toybox.Lang.String) as Toybox.Lang.String",
        ),
        CLASSES.replace(
            "function value(x as Toybox.Lang.Number) as Toybox.Lang.String",
            "function value(x as Toybox.Lang.Number,y as Toybox.Lang.Number) as Toybox.Lang.String",
        ),
    ] {
        assert!(
            compile_sources(&[&classes, "function f(x as A or B){return x.value(2);}"]).is_err()
        );
    }
    for body in [
        "function f(x as A or B or Null){return x.value(2);}",
        "function f(x as A or B) as Toybox.Lang.Boolean{return x.value(2);}",
        "function f(x as A or B){return x.value(\"bad\");}",
    ] {
        assert!(compile_sources(&[CLASSES, body]).is_err(), "{body}");
    }
    let unit = compile_sources(&[CLASSES, "function f(x as A or B){return x.value(2);}"]).unwrap();
    let text = unit.to_ir_text();
    for changed in [
        text.replace(" A|B value", " A value"),
        text.replace(" A|B value", " A|Missing value"),
        text.replace("result=union(integer|string)", "result=integer"),
    ] {
        assert_ne!(text, changed);
        assert!(parse_unit_ir(&changed).is_err(), "{changed}");
    }
}

#[test]
fn shutdown_effects_include_every_target_and_indirect_caller() {
    for invoke in ["x.touch();", "M.touch(x);"] {
        let safe = format!(
            "class A{{function initialize(){{}}function touch() as Void{{}}}} class B{{function initialize(){{}}function touch() as Void{{self.helper();}}function helper() as Void{{}}}} module M{{function touch(x as A or B) as Void{{x.touch();}}}} class Holder{{var keep as Toybox.Lang.Object;function initialize(x as B or A){{{invoke}self.keep=x;}}}}"
        );
        let unit = compile_sources(&[&safe]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
        let unsafe_source = safe.replace(
            "function helper() as Void{}",
            "function helper() as Void{System.exit();}",
        );
        let error = compile_sources(&[&unsafe_source]).unwrap_err();
        assert!(error.to_string().contains("shutdown"), "{error}");
    }
}

#[test]
fn distinct_source_results_are_retained_and_mixed_void_calls_can_be_discarded() {
    let classes = "class A{function initialize(){}function copy() as A{return new A();}function touch() as Void{}}class B{function initialize(){}function copy() as B{return new B();}function touch() as Toybox.Lang.Number{return 9;}}";
    let unit = compile_sources(&[
        classes,
        "function f(x as A or B) as A or B{x.touch();return x.copy();}",
    ])
    .unwrap();
    assert!(unit.to_ir_text().contains("A|B copy result=source-union("));
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert!(compile_sources(&[classes, "function f(x as A or B){return x.touch();}"]).is_err());
}

#[test]
fn contextual_literals_can_satisfy_concrete_and_any_parameters_together() {
    let classes=CLASSES.replace("function accept(x as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number{return x.size();}}", "function accept(x) as Toybox.Lang.Number{return x.size();}}");
    // Keep A concrete while B accepts the dynamic argument.
    let split = classes.find("class B").unwrap();
    let classes = format!(
        "{}{}",
        &CLASSES[..CLASSES.find("class B").unwrap()],
        &classes[split..]
    );
    let unit =
        compile_sources(&[&classes, "function f(x as A or B){return x.accept([]);}"]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}
