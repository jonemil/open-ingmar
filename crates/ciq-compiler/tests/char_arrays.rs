use ciq_compiler::ArrayElement;
use ciq_compiler::{
    ParameterType, ReturnType,
    api::{ApiContext, ApiLevel, Type},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, compile_sources_with_api, compile_with_api, parse_ir,
    parse_ir_with_api, parse_unit_ir_with_api, symbol_bindings,
};

fn context(level: ApiLevel) -> ApiContext {
    ApiContext::new(level, level).unwrap()
}

#[test]
fn char_arrays_preserve_element_contracts_and_complete_pipelines() {
    let ctx = context(ApiLevel(1, 3, 0));
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-char-arrays/app.mc"),
            include_str!("../../../examples/compiler-char-arrays/arrays.mc"),
        ],
        ctx,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), ctx).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [37; 16],
        entry: "start".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    let f = compile("function f(chars as Toybox.Lang.Array<Toybox.Lang.Char>) as Toybox.Lang.Array<Toybox.Lang.Char> { return chars; }").unwrap();
    assert_eq!(
        f.parameter_types(),
        &[ParameterType::Array(ArrayElement::Char)]
    );
    assert_eq!(f.return_type(), ReturnType::Array(ArrayElement::Char));
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let f = compile(
        "function f() as Toybox.Lang.Array<Toybox.Lang.Char> { return \"abc\".toCharArray(); }",
    )
    .unwrap();
    assert_eq!(f.return_type(), ReturnType::Array(ArrayElement::Char));
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let f = compile("function f() { return M.text().toCharArray().size(); }");
    // Unknown standalone user call results default to Number; units resolve String.
    assert!(f.is_err());
    let symbols = ciq_assembly::Program::parse(".symbol toCharArray \"toCharArray\" id=0x008004c4\n.symbol size \"size\" id=0x008000ae\n.symbol indexOf \"indexOf\" id=0x008004bf").unwrap();
    for (source, name, id) in [
        (
            "function f() { return \"A\".toCharArray().size(); }",
            "size",
            0x008000ae,
        ),
        (
            "function f() { return \"A\".toCharArray().indexOf('A'); }",
            "indexOf",
            0x008004bf,
        ),
    ] {
        let f = compile(source).unwrap();
        let bindings = symbol_bindings(&f.lower(), &symbols).unwrap();
        assert_eq!(bindings["__ciq_native_toCharArray"], 0x008004c4);
        assert_eq!(bindings[&format!("__ciq_native_{name}")], id);
    }
}

#[test]
fn char_array_contracts_reject_wrong_elements_receivers_and_unsupported_consumers() {
    for source in [
        "function f(a as Toybox.Lang.Array) { return a.size(); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Object>) { return a.size(); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Char> or Null) { return 0; }",
        "function f() { return 1.toCharArray(); }",
        "function f() { return \"A\".toCharArray(1); }",
        "function f() { return \"A\".toCharArray().size(1); }",
        "function f() { return \"A\".toCharArray().indexOf(); }",
        "function f() { return \"A\".toCharArray().indexOf(65); }",
        "function f() { return \"A\".toCharArray().indexOf(null); }",
        "function f() { return \"A\".toCharArray().indexOf(true); }",
        "function f() { return \"A\".toCharArray().add('B'); }",
        "function f() { return \"A\".toCharArray().reverse(); }",
        "function f() { var a = \"A\".toCharArray(); a = \"x\"; return 0; }",
        "function f() { System.println(\"A\".toCharArray()); return 0; }",
        "function f() { return \"x\" + \"A\".toCharArray(); }",
        "function f() { return \"A\".toCharArray() == \"A\".toCharArray(); }",
        "function f(s as Toybox.Lang.String or Null) { return s.toCharArray().size(); }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let unit = "module A { function take(a as Toybox.Lang.Array<Toybox.Lang.Char>) { return a.size(); } } function start() { return A.take(1); }";
    assert!(compile_sources(&[unit]).is_err());
    let valid = compile("function f(s as Toybox.Lang.String or Null) { if (!(s == null)) { return s.toCharArray().size(); } return 0; }").unwrap();
    assert_eq!(valid, parse_ir(&valid.to_ir_text()).unwrap());
    let array_entry = compile_sources(&[
        "function start() as Toybox.Lang.Array<Toybox.Lang.Char> { return \"A\".toCharArray(); }",
    ])
    .unwrap();
    assert!(
        build_console_application(
            &array_entry,
            &ConsoleApplication {
                id: [1; 16],
                entry: "start".into()
            }
        )
        .is_err()
    );
}

#[test]
fn saved_ir_and_api_context_recheck_array_element_types_and_introduction_levels() {
    let source = "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) { return a.size(); }";
    let f = compile_with_api(source, context(ApiLevel(1, 3, 0))).unwrap();
    assert!(compile_with_api(source, context(ApiLevel(1, 2, 0))).is_err());
    assert!(parse_ir_with_api(&f.to_ir_text(), context(ApiLevel(1, 2, 0))).is_err());
    assert!(parse_ir(&f.to_ir_text().replace("char-array", "string")).is_err());
    let source = "function f() { return \"A\".toCharArray().size(); }";
    assert!(
        compile_with_api(
            source,
            ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap()
        )
        .is_err()
    );
    let unit = compile_sources(&["module A { function convert() as Toybox.Lang.Array<Toybox.Lang.Char> { return \"a\".toCharArray(); } } function start() { return A.convert().size(); }"]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains("result=char-array"));
    assert!(
        parse_unit_ir_with_api(
            &text.replace("result=char-array", "result=string"),
            context(ApiLevel(1, 3, 0))
        )
        .is_err()
    );
    let array = Type::Array(Box::new(Type::named("Toybox.Lang.Char")));
    assert_eq!(ReturnType::Array(ArrayElement::Char).api_type(), array);
    assert!(
        context(ApiLevel(1, 3, 0))
            .check_type(ciq_compiler::api::core(), &array)
            .is_ok()
    );
    assert!(
        context(ApiLevel(1, 0, 0))
            .check_type(ciq_compiler::api::core(), &array)
            .is_err()
    );
}
