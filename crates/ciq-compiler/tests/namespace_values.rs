use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;

#[test]
fn namespace_values_preserve_saved_contracts_and_complete_application() {
    let source = include_str!("../../../examples/compiler-namespace-values/app.mc");
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".module-value"));
    assert!(text.contains(".class-value"));
    assert!(text.contains(".module-self"));
    let inferred =
        ciq_compiler::parse_function_ir("ciq-ir\n.function f\n.module-self %0\n.return %0\n.end\n")
            .unwrap();
    assert_eq!(
        inferred.return_type(),
        ciq_compiler::ReturnType::ModuleValue
    );
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0xb67c1691c9a5422a9d3202a0c016870cu128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
}

#[test]
fn namespaces_keep_value_shadowing_and_remain_distinct_from_instances() {
    for source in [
        "module M{}function f(){return M;}",
        "class C{}function f(){return C;}",
        "function f(){return self;}function g(){return me;}",
        "module M{function f(){return self;}}",
        "using M as A;module M{}function f(){return A;}",
        "using M as A;module M{}function f(A as Toybox.Lang.Number){return A+1;}",
        "module M{class C{}function f(){return C;}}",
        "module M{}module N{module M{}function f(){return $.M!=M;}}",
        "module M{}function f(flag as Toybox.Lang.Boolean){return flag?M:null;}",
        "module M{}class C{}function f(flag as Toybox.Lang.Boolean){var value=M;value=C;return value;}",
        "function f(){return Toybox.Lang;}",
        "using Toybox.Lang as L;function f(){return L.Number;}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}")));
    }
    for source in [
        "module M{}function f() as Toybox.Lang.Object{return M;}",
        "class C{}function f() as C{return C;}",
        "module M{}function f(){return M+1;}",
        "class C{}function f(){System.println(C);}",
        "module M{}function f(){return M.method(:f);}",
        "function f(){return $.Missing;}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}

#[test]
fn saved_namespace_literals_and_receivers_cannot_forge_declarations() {
    let text = compile_sources(&["module M{}class C{}function f(){var m=M;return C;}"])
        .unwrap()
        .to_ir_text();
    for bad in [
        text.replace(" M span=", " Missing span="),
        text.replace(" C span=", " Missing span="),
    ] {
        assert_ne!(text, bad);
        assert!(parse_unit_ir(&bad).is_err());
    }
    assert!(
        ciq_compiler::parse_function_ir(
            "ciq-ir\n.function f receiver=instance\n.module-self %0\n.return %0\n.end\n"
        )
        .is_err()
    );
}

#[test]
fn namespace_lookup_carries_initializer_shutdown_effects() {
    let source = "module M{var X=1;}class C{var value=M;}";
    let text = compile_sources(&[source]).unwrap().to_ir_text();
    let bad = text
        .lines()
        .flat_map(|line| {
            if line.starts_with(".assign-shared M X ") {
                vec![line, ".call-native-module-discard Toybox.System exit"]
            } else {
                vec![line]
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let error = parse_unit_ir(&bad).unwrap_err();
    assert!(error.to_string().contains("shutdown"), "{error}");
}
