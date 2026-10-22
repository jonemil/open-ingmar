use ciq_compiler::{
    Operation, ReturnType, compile_function, compile_sources, parse_function_ir, parse_unit_ir,
};

#[test]
fn empty_dictionary_values_keep_native_text_and_application_paths_equivalent() {
    let f = compile_function("function f() as Toybox.Lang.Dictionary { return {}; }").unwrap();
    assert_eq!(f.return_type(), ReturnType::Dictionary);
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    assert_eq!(f.lower().encode().unwrap(), [0x35, 1, 0x37, 0x2f, 0x16]);
    assert!(matches!(
        f.operations()[0].operation,
        Operation::DictionaryNew { .. }
    ));
    let unit =
        compile_sources(&[include_str!("../../../examples/compiler-dictionary/app.mc")]).unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    let source = include_str!("../../../examples/compiler-dictionary/app.mc");
    assert!(compile_sources(&[&source.replace("if (state != null)", "if (true)")]).is_err());
    assert_eq!(unit, resumed);
    let app = ciq_compiler::application::ConsoleApplication {
        id: [0x45; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        ciq_compiler::application::build_console_application(&unit, &app).unwrap(),
        ciq_compiler::application::build_console_application(&resumed, &app).unwrap()
    );
    let p = unit.lower();
    assert_eq!(p, ciq_assembly::Program::parse(&p.to_text()).unwrap());
}

#[test]
fn dictionary_methods_and_nullable_state_require_checked_non_null_receivers() {
    let api = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(1, 0, 0),
        ciq_compiler::api::ApiLevel(1, 0, 0),
    )
    .unwrap();
    let source = "function f() as Toybox.Lang.Number { return {}.size(); }";
    let f = ciq_compiler::compile_function_with_api(source, api).unwrap();
    assert_eq!(
        f,
        ciq_compiler::parse_function_ir_with_api(&f.to_ir_text(), api).unwrap()
    );
    for source in [
        "function f(d as Toybox.Lang.Dictionary) as Toybox.Lang.Number { return d.size(); }",
        "function f() as Toybox.Lang.Boolean { return {}.isEmpty(); }",
        "function f() as Toybox.Lang.Number { var d = {}; d = {}; return d.size(); }",
    ] {
        let f = compile_function(source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    for source in [
        "function f() as Toybox.Lang.Number { var d = {}; return d.get(\"one\"); }",
        "function f() as Toybox.Lang.Number { var d = {}; d.put(\"one\", 1); return d.size(); }",
        "function f() as Toybox.Lang.Number { var d = {}; return d.size(1); }",
        "function f() as Toybox.Lang.Number { var d = {}; d = 1; return d.size(); }",
        "function f() as Toybox.Lang.Number { return {} + {}; }",
    ] {
        assert!(compile_function(source).is_err(), "accepted {source}");
    }
    let input =
        "ciq-ir\n.function f\n.dictionary-new %0\n.call-native %1 %0 size\n.return %1\n.end\n";
    let f = parse_function_ir(input).unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    assert!(parse_function_ir(&input.replace("%1 %0 size", "%1 %99 size")).is_err());
    assert!(parse_function_ir(&input.replace(".dictionary-new %0", ".null %0")).is_err());
    assert!(
        parse_function_ir(&input.replace(".dictionary-new %0", ".dictionary-new %0 1")).is_err()
    );
}
