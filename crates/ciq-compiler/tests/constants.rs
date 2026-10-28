use ciq_compiler::{compile_function, compile_sources, parse_unit_ir};
mod common;
#[test]
fn constants_keep_scope_and_saved_pipeline_values() {
    let sources = [
        include_str!("../../../examples/compiler-constants/app.mc"),
        include_str!("../../../examples/compiler-constants/values.mc"),
    ];
    let unit = compile_sources(&sources).unwrap();
    let options = ciq_compiler::application::ConsoleApplication {
        id: 0x7a2be25116cc441ea611855fba1de319u128.to_be_bytes(),
        entry: "MinimalConsoleApp.onStart".into(),
    };
    common::application(&unit, &options);
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    common::pipeline(compile_sources(&[sources[1], sources[0]]).unwrap());
    let fragment = compile_function("const X=41;function f(){return X;}").unwrap();
    assert_eq!(
        fragment.lower(),
        compile_function("function f(){return 41;}")
            .unwrap()
            .lower()
    );
    assert!(compile_function("const X=40+1;function f(){return X;}").is_err());
}
#[test]
fn constant_errors_and_rebinding_are_rejected() {
    for source in [
        "const X=Missing;function f(){}",
        "const X=1;const X=2;function f(){}",
        "const X=1;function X(){}",
        "class X{}const X=1;function f(){}",
        "module M{const X=1;var X;function f(){}}",
        "const X=1;function f(){X=2;}",
        "const X=1;function f(){X+=2;}",
        "const X=1;function f(){X++;}",
        "module M{const X=1;}function f(){M.X=2;}",
        "module M{const X=1;}function f(){M.X++;}",
        "class C{const X=1;function f(){self.X=2;}}",
        "class C{const X=1;}function f(){return C.X;}",
        "class C{const initialize=1;}",
        "class C{const toString=1;}",
        "function f(){const X=1;return X;}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let source = "const UNUSED='A';function f(){}";
    let low = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(1, 0, 0),
        ciq_compiler::api::ApiLevel(1, 0, 0),
    )
    .unwrap();
    assert!(ciq_compiler::compile_sources_with_api(&[source], low).is_err());
}

#[test]
fn constant_counts_are_bounded_and_aliases_do_not_require_recursive_expansion() {
    let mut source = String::new();
    for n in 0..256 {
        source.push_str(&format!("const X{n}={n};"));
    }
    source.push_str("function f(){return X255;}");
    assert!(compile_sources(&[&source]).is_ok());
    source.push_str("const EXTRA=1;");
    assert!(compile_sources(&[&source]).is_err());
    let mut chain = String::new();
    for n in 0..65 {
        chain.push_str(&format!("const X{n}=X{};", n + 1));
    }
    chain.push_str("const X65=1;function f(){return X0;}");
    common::pipeline(compile_sources(&[&chain]).unwrap());
}
