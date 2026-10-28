use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;
#[test]
fn shared_constants_keep_complete_pipeline_and_saved_ownership() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-shared-constants/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".shared-constant $ TEXT string"));
    assert!(text.contains(".shared-constant M ITEMS number-array"));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x9df9279337df4bc79c190bd70f6d2121u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
}

#[test]
fn readonly_writes_hidden_calls_and_initialization_dependencies_are_checked() {
    for source in [
        "const X=[1];function f(){X=[2];}",
        "const X=1l;function f(){X++;}",
        "const X=1l;function f(){X+=1l;}",
        "module M{const X=[1];function f(){self.X=[2];}}",
        "module M{const X=[1];}function f(){M.X=[2];}",
        "module M{const X=1l;}function f(){M.X++;}",
        "const X=[1];function f(){__ciq_initialize_shared_X();}",
        "const X=[1];const X=[2];function f(){}",
        "const X=[1];function X(){}",
        "const X=missing;function f(){}",
        "const X=f();function f() as Toybox.Lang.Array<Toybox.Lang.Number>{return X;}",
        "const X=f();function f() as Toybox.Lang.Array<Toybox.Lang.Number>{return [1];}",
        "module M{const X=f();function f() as Toybox.Lang.Array<Toybox.Lang.Number>{return [1];}}",
        "const X=f();const Y=[1];function f() as Toybox.Lang.Array<Toybox.Lang.Number>{return Y;}",
        "const X=M.f();module M{const Y=g();function f() as Toybox.Lang.Array<Toybox.Lang.Number>{return Y;}}function g() as Toybox.Lang.Array<Toybox.Lang.Number>{return X;}",
        "const X=f();function f() as Toybox.Lang.Array<Toybox.Lang.Number>{Toybox.System.exit();return [1];}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let unit = compile_sources(&["const X=[1];function f(){} module M{const Y=\"x\";}"]).unwrap();
    let text = unit.to_ir_text();
    let missing = text
        .lines()
        .filter(|line| !line.starts_with(".assign-shared $ X "))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(parse_unit_ir(&missing).is_err());
    let twice = text.replace(".assign-shared $ X", ".assign-shared M Y");
    assert!(parse_unit_ir(&twice).is_err());
    let absent = text
        .lines()
        .filter(|line| !line.starts_with(".shared-constant $ X "))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(parse_unit_ir(&absent).is_err());
    let forged = text.replace(
        ".function f result=any\n",
        ".function f result=any\n.string %999 \"bad\"\n.assign-shared M Y %999\n",
    );
    assert_ne!(forged, text);
    assert!(parse_unit_ir(&forged).is_err());
    assert!(
        ciq_compiler::compile_function("const X=[1];function f(){return X;}")
            .unwrap_err()
            .message
            .contains("compile_sources")
    );
}

#[test]
fn reopened_owners_shadowing_and_unused_api_checks_keep_their_contracts() {
    let sources = [
        "using Toybox.Lang;const X=[1]; module M{const X=\"module\";} function a() as Lang.Array<Lang.Number>{return X;}",
        "using Toybox.Lang;module M{function own() as Lang.String{return self.X;}function local(X as Lang.String) as Lang.String{return X;}}function root(X as Lang.String) as Lang.String{return X;}",
    ];
    common::pipeline(compile_sources(&sources).unwrap());
    let low = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(1, 0, 0),
        ciq_compiler::api::ApiLevel(1, 0, 0),
    )
    .unwrap();
    assert!(
        ciq_compiler::compile_sources_with_api(&["const UNUSED=[1]b;function f(){}"], low).is_err()
    );
    let unit = compile_sources(&["const X=[1];function f(){return X;}"]).unwrap();
    let text = unit.to_ir_text();
    assert!(parse_unit_ir(&text.replace("X result=number-array", "X result=string")).is_err());
}

#[test]
fn lazy_module_initialization_effects_reach_class_readiness_checks() {
    let unit=compile_sources(&["module M{const TEXT=\"x\";} class C{var value as Toybox.Lang.String;function initialize(){self.value=M.TEXT;}} "]).unwrap();
    let text = unit.to_ir_text();
    let mut lines = Vec::new();
    for line in text.lines() {
        lines.push(line.to_owned());
        if line.starts_with(".assign-shared M TEXT ") {
            lines.push(".call-native-module-discard Toybox.System exit".into());
        }
    }
    let error = parse_unit_ir(&lines.join("\n")).unwrap_err();
    assert!(error.message.contains("shutdown"), "{error}");
}

#[test]
fn value_receivers_shadow_same_named_module_storage() {
    let source = "using Toybox.Lang;module M{const VALUE=\"module\";} class Token{const VALUE=\"instance\";} class C{const M=new Token();function text() as Lang.String{return M.VALUE;}} function f() as Lang.String{return new C().text();}";
    let unit = compile_sources(&[source]).unwrap();
    let method = unit
        .classes()
        .iter()
        .find(|c| c.name == "C")
        .unwrap()
        .methods
        .iter()
        .find(|m| m.name() == "text")
        .unwrap();
    assert!(
        !method
            .operations()
            .iter()
            .any(|op| matches!(op.operation, ciq_compiler::Operation::SharedRead { .. }))
    );
    common::pipeline(unit);
}

#[test]
fn qualified_stored_objects_keep_nested_member_paths() {
    let source = "using Toybox.Lang;class Token{const VALUE=\"instance\";}module M{const TOKEN=new Token();}function f() as Lang.String{return M.TOKEN.VALUE;}";
    // Infer the module slot's class identity before resolving its field chain.
    let unit = compile_sources(&[source]).unwrap();
    common::pipeline(unit);
}
