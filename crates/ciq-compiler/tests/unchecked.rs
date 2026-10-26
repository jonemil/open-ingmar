use ciq_compiler::{CompileOptions, TypeCheckLevel, compile_sources, compile_sources_with_options};
mod common;
fn off() -> CompileOptions {
    CompileOptions {
        type_check: TypeCheckLevel::Off,
        ..Default::default()
    }
}
#[test]
fn unchecked_value_uses_resume_as_validated_assertions() {
    let fragment = ciq_compiler::compile_function_with_options(
        "function f(x as Toybox.Lang.String) as Toybox.Lang.Number {return x-1;}",
        off(),
    )
    .unwrap()
    .artifact;
    assert_eq!(
        fragment,
        ciq_compiler::parse_function_ir(&fragment.to_ir_text()).unwrap()
    );
    for source in [
        "function f() as Toybox.Lang.Number {return \"text\";}",
        "function f(x as Toybox.Lang.String) as Toybox.Lang.Number {return x-1;}",
        "function f(x as Toybox.Lang.String) {var n=x-1;return n;}",
        "function f(x as Toybox.Lang.String) as Toybox.Lang.Number {++x;return x;}",
        "function f(x as Toybox.Lang.String) as Toybox.Lang.Number {return -x;}",
        "function f(x as Toybox.Lang.String) as Toybox.Lang.Number {return ~x;}",
        "function f(x as Toybox.Lang.String) as Toybox.Lang.Boolean {return x<1;}",
        "function f(x as Toybox.Lang.Number) as Toybox.Lang.Number {if(x){return 41;}return 0;}",
        "function f(x as Toybox.Lang.Number) as Toybox.Lang.Number {x=\"x\";return x;}",
        "function take(x as Toybox.Lang.Number) as Toybox.Lang.Number {return x;} function f(){return take(\"x\");}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>,i as Toybox.Lang.String){return a[i];}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>){a[\"x\"]=true;}",
        "class A {} class B {} function f() as A {return new B();}",
        "function f() {return \"x\".substring(true,false);}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "checked: {source}");
        let report = compile_sources_with_options(&[source], off())
            .unwrap_or_else(|e| panic!("{source}: {e}"));
        assert!(report.warnings.is_empty());
        assert!(report.artifact.to_ir_text().contains(".cast "));
        common::pipeline(report.artifact);
    }
}
#[test]
fn unchecked_lowering_keeps_declaration_arity_api_and_stack_errors() {
    for source in [
        "function f(){return missing;}",
        "function f(x,x){return 1;}",
        "function f(){return new Missing();}",
        "function f(){return missing();}",
        "function take(x){return x;} function f(){return take();}",
        "function f(){return \"x\".length(1);}",
        "function f() as Void {} function g(){return f();}",
        "class C {var x as Toybox.Lang.String;function initialize(){}}",
    ] {
        assert!(
            compile_sources_with_options(&[source], off()).is_err(),
            "{source}"
        );
    }
    let mut options = off();
    options.api = Some(
        ciq_compiler::api::ApiContext::new(
            ciq_compiler::api::ApiLevel(5, 2, 0),
            ciq_compiler::api::ApiLevel(1, 0, 0),
        )
        .unwrap(),
    );
    assert!(
        compile_sources_with_options(&["function f(){return \"x\".substring(0,null);}"], options)
            .is_err()
    );
    assert!(
        compile_sources_with_options(&["function f(){return (\"1\"+\"2\").toDouble();}"], options)
            .is_err()
    );
    common::pipeline(
        compile_sources_with_options(&["function f(){return \"x\".substring(0,1);}"], options)
            .unwrap()
            .artifact,
    );
}
#[test]
fn unchecked_receivers_keep_dynamic_lookup_and_reject_unknown_selectors() {
    let declarations =
        "class C {} class D {var absent=1;function missing() as Toybox.Lang.Number {return 41;}}";
    for body in [
        "function f(x as Toybox.Lang.Number){return x.length();}",
        "function f(x as C){return x.missing();}",
        "function f(x as C){x.missing();}",
        "function f(x as Toybox.Lang.Number){return x[0];}",
        "function f(x as Toybox.Lang.Number){x[0]=41;}",
        "function f(x as C){return x.absent;}",
        "function f(x as C){var value=x.absent;return value;}",
        "function f(x as C){x.absent=41;return x.absent;}",
        "function f(x as C){x.absent+=1;return x.absent;}",
        "function f(x as C){x.absent=true?41:42;return x.absent;}",
        "function f(x as D or Null){return x.absent;}",
    ] {
        assert!(
            compile_sources(&[declarations, body]).is_err(),
            "checked: {body}"
        );
        common::pipeline(
            compile_sources_with_options(&[declarations, body], off())
                .unwrap_or_else(|e| panic!("{body}: {e}"))
                .artifact,
        );
    }
    for body in [
        "function f(x as Toybox.Lang.Number){return x.undefinedMember();}",
        "function f(x as C){return x.undefinedMember();}",
        "function f(x as C){return x.undefinedField;}",
        "function f(x as C){x.undefinedField=1;}",
    ] {
        assert!(
            compile_sources_with_options(&[declarations, body], off()).is_err(),
            "{body}"
        );
    }
}

#[test]
fn unchecked_receiver_erasure_does_not_bypass_metadata_or_initialization() {
    let declarations = "class C {} class D {var absent=1;function quit() as Void {System.exit();}}";
    let early = "class Box {var required as Toybox.Lang.String;function initialize(x as C){x.quit();self.required=\"set\";}}";
    assert!(
        compile_sources_with_options(&[declarations, early], off())
            .unwrap_err()
            .to_string()
            .contains("shutdown")
    );
    let safe = early.replace(
        "x.quit();self.required=\"set\";",
        "self.required=\"set\";x.quit();",
    );
    common::pipeline(
        compile_sources_with_options(&[declarations, &safe], off())
            .unwrap()
            .artifact,
    );
    let unit = compile_sources_with_options(
        &[declarations, "function f(x as C){return x.absent;}"],
        off(),
    )
    .unwrap()
    .artifact;
    let text = unit.to_ir_text();
    let read = text
        .lines()
        .find(|l| l.starts_with(".member-read "))
        .unwrap();
    let forged = read.replace(" absent", " absent field=integer");
    assert_ne!(read, forged);
    let error = ciq_compiler::parse_unit_ir(&text.replace(read, &forged)).unwrap_err();
    assert!(error.to_string().contains("source field access"), "{error}");
}

#[test]
fn unchecked_application_preserves_all_saved_stages() {
    let report = compile_sources_with_options(
        &[include_str!("../../../examples/compiler-unchecked/app.mc")],
        off(),
    )
    .unwrap();
    common::application(
        &report.artifact,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x58cdd61b9fd04ab8ba564323684a418bu128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
}
