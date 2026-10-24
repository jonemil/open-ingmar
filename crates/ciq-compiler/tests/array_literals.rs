use ciq_compiler::ArrayElement;
use ciq_compiler::{
    Function, LocatedOperation, Operation, ParameterType, ReturnType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_function_with_api, compile_sources_with_api, parse_function_ir,
    parse_function_ir_with_api, parse_unit_ir_with_api,
};

fn context() -> ApiContext {
    ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 3, 0)).unwrap()
}

#[test]
fn array_literals_preserve_complete_pipelines_and_allocate_before_elements() {
    let unit = compile_sources_with_api(
        &[include_str!(
            "../../../examples/compiler-array-literals/app.mc"
        )],
        context(),
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [43; 16],
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
    let source = "function f(c as Toybox.Lang.Char) as Toybox.Lang.Array<Toybox.Lang.Char> { return [c, 'b']; }";
    let f = compile_function(source).unwrap();
    assert_eq!(f.return_type(), ReturnType::Array(ArrayElement::Char));
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    let ops = f.operations();
    let allocation = ops
        .iter()
        .position(|op| {
            matches!(
                op.operation,
                Operation::ArrayBegin { .. } | Operation::ArrayConstant { .. }
            )
        })
        .unwrap();
    let element = ops
        .iter()
        .position(|op| matches!(op.operation, Operation::ParameterRead { .. }))
        .unwrap();
    assert!(allocation < element);
    let span = ops[allocation].span.unwrap();
    assert_eq!(&source[span.start..span.end], "[c, 'b']");
    assert_eq!(
        ops.iter()
            .filter(|op| matches!(op.operation, Operation::LocalInitialize { .. }))
            .count(),
        1
    );
    let f = compile_function("function f() as Toybox.Lang.Char { return [(true || false).hashCode().toChar(), 'a'][0]; }").unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
}

#[test]
fn array_literals_enforce_element_api_and_resource_limits() {
    for elements in ["1", "null", "true", "'a', 1", "['a']", "'a',,"] {
        let source = format!(
            "function f() as Toybox.Lang.Array<Toybox.Lang.Char> {{ return [{elements}]; }}"
        );
        assert!(compile_function(&source).is_err(), "{source}");
    }
    let source = "function f() as Toybox.Lang.Array<Toybox.Lang.Char> { return ['a']; }";
    let f = compile_function_with_api(source, context()).unwrap();
    let old = ApiContext::new(ApiLevel(1, 2, 0), ApiLevel(1, 2, 0)).unwrap();
    assert!(compile_function_with_api(source, old).is_err());
    assert!(parse_function_ir_with_api(&f.to_ir_text(), old).is_err());
    let list = |count| {
        format!(
            "function f() as Toybox.Lang.Array<Toybox.Lang.Char> {{ return [{}]; }}",
            vec!["'a' + 0"; count].join(",")
        )
    };
    compile_function(&list(100)).unwrap();
    assert!(
        compile_function(&list(1020))
            .unwrap_err()
            .message
            .contains("1024-operation")
    );
    let nested = format!(
        "function f() as Toybox.Lang.Number {{ return {}'a'{}; }}",
        "[".repeat(65),
        "]".repeat(65)
    );
    assert!(
        compile_function(&nested)
            .unwrap_err()
            .message
            .contains("nesting")
    );
}

#[test]
fn empty_typed_ir_allocation_has_verified_bytes_and_rejects_unsupported_elements() {
    let text = "ciq-ir\n.function f\n.array-new %0 char\n.return %0\n.end\n";
    let f = parse_function_ir(text).unwrap();
    assert_eq!(f.return_type(), ReturnType::Array(ArrayElement::Char));
    assert_eq!(f.lower().encode().unwrap(), [0x35, 1, 0x37, 0x14, 0x16]);
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    for bad in [".array-new %0 char 1", ".array-new char"] {
        assert!(parse_function_ir(&text.replace(".array-new %0 char", bad)).is_err());
    }
    let numbers = parse_function_ir(&text.replace("char", "number")).unwrap();
    assert_eq!(
        numbers.return_type(),
        ReturnType::Array(ArrayElement::Number)
    );
    assert_eq!(
        numbers.lower().encode().unwrap(),
        f.lower().encode().unwrap()
    );
}

#[test]
fn allocation_is_order_sensitive_even_when_its_result_is_consumed_later() {
    let f =
        compile_function("function f(n as Toybox.Lang.Number) as Toybox.Lang.Array<Toybox.Lang.Char> { n.abs(); return ['a']; }")
            .unwrap();
    let mut ops: Vec<LocatedOperation> = f.operations().to_vec();
    let allocation = ops
        .iter()
        .position(|op| matches!(op.operation, Operation::ArrayConstant { .. }))
        .unwrap();
    let call = ops
        .iter()
        .position(|op| matches!(op.operation, Operation::NativeCall { .. }))
        .unwrap();
    let moved = ops.remove(allocation);
    ops.insert(call, moved);
    let error = Function::with_signature(
        "bad".into(),
        vec![ParameterType::Integer],
        ReturnType::Array(ArrayElement::Char),
        ops,
    )
    .unwrap_err();
    assert!(error.message.contains("evaluation order"), "{error:?}");
}
