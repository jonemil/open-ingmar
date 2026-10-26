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
