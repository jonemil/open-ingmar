use ciq_compiler::{
    ArrayElement, ParameterType, ReturnType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_function_with_api, compile_sources, compile_sources_with_api,
    parse_function_ir, parse_function_ir_with_api, parse_unit_ir_with_api, symbol_bindings,
};

fn context(level: ApiLevel) -> ApiContext {
    ApiContext::new(level, level).unwrap()
}

#[test]
fn number_arrays_preserve_complete_pipelines_and_numeric_read_results() {
    let ctx = context(ApiLevel(1, 3, 0));
    let unit = compile_sources_with_api(
        &[include_str!(
            "../../../examples/compiler-number-arrays/app.mc"
        )],
        ctx,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), ctx).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [44; 16],
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
    let f = compile_function("function f(a as Toybox.Lang.Array<Toybox.Lang.Number>, n as Toybox.Lang.Number) as Toybox.Lang.Number { return a[n]; }")
        .unwrap();
    assert_eq!(
        f.parameter_types(),
        &[
            ParameterType::Array(ArrayElement::Number),
            ParameterType::Integer
        ]
    );
    assert_eq!(f.return_type(), ReturnType::Integer);
    assert_eq!(
        f.lower().encode().unwrap(),
        [0x35, 3, 0x12, 1, 0x12, 2, 0x10, 0x16]
    );
    let f = compile_function("function f(s as Toybox.Lang.String) as Toybox.Lang.Array<Toybox.Lang.Number> { return s.toUtf8Array(); }").unwrap();
    assert_eq!(f.return_type(), ReturnType::Array(ArrayElement::Number));
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    let symbols =
        ciq_assembly::Program::parse(".symbol utf8 \"toUtf8Array\" id=0x008004c5").unwrap();
    assert_eq!(
        symbol_bindings(&f.lower(), &symbols).unwrap()["__ciq_native_toUtf8Array"],
        0x008004c5
    );
}

#[test]
fn invariant_element_types_are_rechecked_in_source_and_saved_ir() {
    for source in [
        "function f() as Void { \"A\".toUtf8Array().add('A'); }",
        "function f() as Void { \"A\".toUtf8Array().indexOf(null); }",
        "function f() as Void { \"A\".toUtf8Array().remove(true); }",
        "function f() as Void { \"A\".toUtf8Array().removeAll(\"A\"); }",
        "function f() as Void { \"A\".toUtf8Array().addAll(\"A\".toCharArray()); }",
        "function f() as Void { \"A\".toCharArray().addAll(\"A\".toUtf8Array()); }",
        "function f(s as Toybox.Lang.String or Null) as Void { s.toUtf8Array(); }",
        "function f() as Void { 1.toUtf8Array(); }",
        "function f() as Void { \"A\".toUtf8Array(1); }",
        "function f() as Void { System.println(\"A\".toUtf8Array()); }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    let f = compile_function("function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { return a[0] - 1; }")
        .unwrap();
    assert!(parse_function_ir(&f.to_ir_text().replace("number-array", "char-array")).is_err());
    let source = "module M { function make() as Toybox.Lang.Array<Toybox.Lang.Number> { return \"a\".toUtf8Array(); } } function start() as Toybox.Lang.Number { return M.make().size(); }";
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains("result=number-array"));
    assert!(
        parse_unit_ir_with_api(
            &text.replace("result=number-array", "result=char-array"),
            context(ApiLevel(1, 3, 0))
        )
        .is_err()
    );
    let bad_call = "module M { function count(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { return a.size(); } } function start() as Toybox.Lang.Number { return M.count(['a']); }";
    assert!(compile_sources(&[bad_call]).is_err());
    let f = compile_function("function f(a as Toybox.Lang.Array<Toybox.Lang.Number>, n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { if (n == null) { return 0; } return a[n] - 1; }").unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
}

#[test]
fn number_element_availability_is_distinct_from_conversion_and_mutation() {
    let initial = context(ApiLevel(1, 0, 0));
    let source = "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { return a.size(); }";
    let f = compile_function_with_api(source, initial).unwrap();
    assert_eq!(
        f,
        parse_function_ir_with_api(&f.to_ir_text(), initial).unwrap()
    );
    let empty = "ciq-ir\n.function empty\n.array-new %0 number\n.return %0\n.end\n";
    let f = parse_function_ir_with_api(empty, initial).unwrap();
    assert_eq!(f.return_type(), ReturnType::Array(ArrayElement::Number));
    assert_eq!(f.lower().encode().unwrap(), [0x35, 1, 0x37, 0x14, 0x16]);
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    let conversion = "function f() as Toybox.Lang.Number { return \"a\".toUtf8Array().size(); }";
    assert!(compile_function_with_api(conversion, context(ApiLevel(1, 2, 0))).is_err());
    compile_function_with_api(conversion, context(ApiLevel(1, 3, 0))).unwrap();
    assert!(
        compile_function_with_api(
            "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Void { a.add(1); }",
            initial
        )
        .is_err()
    );
    let unit = compile_sources(&[
        "function start() as Toybox.Lang.Array<Toybox.Lang.Number> { return \"a\".toUtf8Array(); }",
    ])
    .unwrap();
    assert!(
        build_console_application(
            &unit,
            &ConsoleApplication {
                id: [44; 16],
                entry: "start".into()
            }
        )
        .is_err()
    );
}
