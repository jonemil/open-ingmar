use ciq_compiler::{
    ArrayElement, Function, NativeClass, Operation, ParameterType, ReturnType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_function_with_api, compile_sources, parse_function_ir,
    parse_function_ir_with_api, parse_unit_ir, symbol_bindings,
};

#[test]
fn native_construction_and_object_arrays_preserve_complete_pipelines() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-native-views/app.mc"
    )])
    .unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    let options = ConsoleApplication {
        id: [47; 16],
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
    for source in [
        "function f() as Toybox.WatchUi.View { return new Toybox.WatchUi.View(); }",
        "using Toybox.WatchUi as Ui; function f() as Toybox.WatchUi.View { return new Ui.View(); }",
        "import Toybox.WatchUi; function f() as Toybox.WatchUi.View { return new View(); }",
        "function f(v as Toybox.WatchUi.View) as Toybox.WatchUi.View { return v; }",
        "function f(v as Toybox.WatchUi.View) as Toybox.Lang.Array<Toybox.WatchUi.View> { return [v]; }",
        "function f(a as Toybox.Lang.Array<Toybox.WatchUi.View>) as Toybox.WatchUi.View { return a[0]; }",
        "function f() as Toybox.Lang.Array<Toybox.WatchUi.View> { return []; }",
    ] {
        let f = compile_function(source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    let f = compile_function(
        "function f(v as Toybox.WatchUi.View) as Toybox.WatchUi.View { return v; }",
    )
    .unwrap();
    assert_eq!(
        f.parameter_types(),
        &[ParameterType::Object(NativeClass::View)]
    );
    assert_eq!(f.return_type(), ReturnType::Object(NativeClass::View));
    let symbols = ciq_assembly::Program::parse(".symbol ui \"Toybox_WatchUi\" id=0x00800011\n.symbol view \"View\" id=0x0080019f\n.symbol initialize \"initialize\" id=0x0080005e\n").unwrap();
    let f = compile_function(
        "function f() as Toybox.WatchUi.View { return new Toybox.WatchUi.View(); }",
    )
    .unwrap();
    let bindings = symbol_bindings(&f.lower(), &symbols).unwrap();
    let bytes = f
        .lower()
        .encode_with_references(&bindings, &Default::default())
        .unwrap();
    assert!(
        ciq_mc::decode(&bytes)
            .unwrap()
            .iter()
            .any(|op| op.instruction.opcode() == ciq_mc::Opcode::Newc)
    );
}

#[test]
fn constructed_references_keep_invariant_contracts_and_observable_order() {
    for source in [
        "function f() as Void { var v = new View(); }",
        "function f(WatchUi as Toybox.Lang.Number) as Void { var v = new WatchUi.View(); }",
        "function f(Toybox as Toybox.Lang.Number) as Void { var v = new Toybox.WatchUi.View(); }",
        "function f() as Void { var v = new Toybox.WatchUi.Unknown(); }",
        "function f() as Void { var v = new Toybox.WatchUi.View(1); }",
        "function f() as Void { var new = 1; }",
        "function f() as Toybox.Lang.Array<Toybox.WatchUi.View> { return [new Toybox.WatchUi.View(), 1]; }",
        "function f() as Toybox.Lang.Array<Toybox.WatchUi.View> { return [1, new Toybox.WatchUi.View()]; }",
        "function f(a as Toybox.Lang.Array<Toybox.WatchUi.View>) as Void { a.add(1); }",
        "function f(a as Toybox.Lang.Array<Toybox.WatchUi.View>) as Void { a.addAll(['a']); }",
        "function f(v as Toybox.WatchUi.View) as Void { v = null; }",
        "function f() as Toybox.Lang.Number { return new Toybox.WatchUi.View(); }",
        "function f(v as Toybox.WatchUi.View) as Void { System.println(v); }",
        "function f(v as Toybox.WatchUi.View) as Toybox.Lang.Number { return v.hashCode(); }",
        "function f(v as Toybox.WatchUi.View or Null) as Toybox.Lang.String { return v.toString(); }",
        "import Toybox.WatchUi; function f(View as Toybox.Lang.Number) as Toybox.Lang.Number { return 0; }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    let f =
        compile_function("function f(v as Toybox.WatchUi.View) as Toybox.Lang.Array<Toybox.WatchUi.View> { return [v]; }")
            .unwrap();
    let text = f.to_ir_text();
    let changed = text
        .lines()
        .map(|line| {
            if line.starts_with(".array-begin ") {
                line.replace("view", "number")
            } else {
                line.into()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(changed, text.trim_end());
    assert!(parse_function_ir(&changed).is_err());
    let unit = compile_sources(&["module M { function make() as Toybox.WatchUi.View { return new Toybox.WatchUi.View(); } } function start() as Toybox.Lang.Number { var a = [M.make()]; return a.size(); }"]).unwrap();
    assert!(parse_unit_ir(&unit.to_ir_text().replace("result=view", "result=integer")).is_err());
    let f =
        compile_function("function f() as Void { 1.abs(); var view = new Toybox.WatchUi.View(); }")
            .unwrap();
    let mut operations = f.operations().to_vec();
    let at = operations
        .iter()
        .position(|op| matches!(op.operation, Operation::NativeNew { .. }))
        .unwrap();
    let call = operations
        .iter()
        .position(|op| matches!(op.operation, Operation::NativeCall { .. }))
        .unwrap();
    let allocation = operations.remove(at);
    operations.insert(call, allocation);
    assert!(
        Function::with_signature("bad".into(), Vec::new(), ReturnType::Void, operations)
            .unwrap_err()
            .message
            .contains("evaluation order")
    );
    let text = "ciq-ir\n.function f\n.new-native %0 Toybox.WatchUi.Unknown\n.return %0\n.end\n";
    assert!(parse_function_ir(text).is_err());
}

#[test]
fn constructor_api_is_distinct_from_signature_and_empty_array_availability() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let signature = "function f(v as Toybox.WatchUi.View) as Toybox.WatchUi.View { return v; }";
    let f = compile_function_with_api(signature, initial).unwrap();
    assert_eq!(
        f,
        parse_function_ir_with_api(&f.to_ir_text(), initial).unwrap()
    );
    let source = "function f() as Toybox.WatchUi.View { return new Toybox.WatchUi.View(); }";
    assert!(compile_function_with_api(source, initial).is_err());
    let context = ApiContext::new(ApiLevel(2, 1, 0), ApiLevel(2, 1, 0)).unwrap();
    let f = compile_function_with_api(source, context).unwrap();
    assert_eq!(
        f,
        parse_function_ir_with_api(&f.to_ir_text(), context).unwrap()
    );
    assert!(parse_function_ir_with_api(&f.to_ir_text(), initial).is_err());
    let f = compile_function_with_api(
        "function f() as Toybox.Lang.Array<Toybox.WatchUi.View> { return []; }",
        initial,
    )
    .unwrap();
    assert_eq!(
        f.return_type(),
        ReturnType::Array(ArrayElement::Object(NativeClass::View))
    );
    let unit = compile_sources(&[
        "function start() as Toybox.WatchUi.View { return new Toybox.WatchUi.View(); }",
    ])
    .unwrap();
    assert!(
        build_console_application(
            &unit,
            &ConsoleApplication {
                id: [47; 16],
                entry: "start".into()
            }
        )
        .is_err()
    );
}
