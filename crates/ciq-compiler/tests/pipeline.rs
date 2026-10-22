use ciq_compiler::{
    Function, LocatedOperation, Operation, Span, ValueId, compile_function, parse_function_ir,
};

#[test]
fn all_literal_pipelines_preserve_ir_labels_and_bytes() {
    for literal in 0..=127 {
        let source = format!("function probe() {{ return {literal}; }}");
        let ir = compile_function(&source).unwrap();
        let reloaded_ir = parse_function_ir(&ir.to_ir_text()).unwrap();
        assert_eq!(ir, reloaded_ir);
        let model = ir.lower();
        let reloaded_model = ciq_assembly::Program::parse(&model.to_text()).unwrap();
        assert_eq!(model, reloaded_model);
        assert_eq!(model.labels().unwrap(), reloaded_model.labels().unwrap());
        assert_eq!(model.encode().unwrap(), reloaded_model.encode().unwrap());
        assert_eq!(
            model.encode().unwrap(),
            reloaded_ir.lower().encode().unwrap()
        );
    }
}

#[test]
fn edited_ir_uses_explicit_value_ids_and_optional_source_metadata() {
    let text = "ciq-ir\n.function probe\n.integer %17 42\n.return %17\n.end\n";
    let ir = parse_function_ir(text).unwrap();
    assert_eq!(ir.operations()[0].span, None);
    assert_eq!(ir.lower().encode().unwrap(), [0x35, 1, 0x38, 42, 0x16]);
    assert_eq!(parse_function_ir(&ir.to_ir_text()).unwrap(), ir);
    let error = parse_function_ir(&text.replace(".return %17", ".return %18")).unwrap_err();
    assert_eq!(error.line, 4);
    assert!(error.message.contains("defined value"));
}

#[test]
fn text_and_constructed_ir_share_semantic_validation() {
    let integer = |value| LocatedOperation {
        operation: Operation::Integer {
            result: ValueId(0),
            value,
        },
        span: None,
    };
    let returned = |value| LocatedOperation {
        operation: Operation::Return {
            value: ValueId(value),
        },
        span: None,
    };
    for operations in [
        vec![],
        vec![integer(41)],
        vec![integer(41), returned(1)],
        vec![returned(0), integer(41)],
        vec![integer(41), integer(42), returned(0)],
        vec![
            integer(41),
            LocatedOperation {
                operation: Operation::Return { value: ValueId(0) },
                span: Some(Span { start: 7, end: 3 }),
            },
        ],
    ] {
        assert!(Function::new("probe".into(), operations).is_err());
    }
    for text in [
        "ciq-ir unexpected\n.function probe\n.integer %0 41\n.return %0\n.end",
        "ciq-ir\n.function probe\n.integer %0 2147483648\n.return %0\n.end",
        "ciq-ir\n.function probe\n.integer %0 41\n.integer %0 42\n.return %0\n.end",
        "ciq-ir\n.function probe\n.integer %0 41 span=7:3\n.return %0\n.end",
        "ciq-ir\n.function probe\n.integer %0 41\n.return %0",
        "ciq-ir\n.function probe\n.integer %0 41\n.return %0\n.end\n.return %0",
        "ciq-ir\n.function probe\n.integer %4294967296 41\n.return %0\n.end",
    ] {
        assert!(parse_function_ir(text).is_err(), "{text}");
    }
    assert!(Function::new("invalid.name".into(), vec![integer(41), returned(0)]).is_err());
}

#[test]
fn numbered_headers_are_rejected() {
    let function = "\n.function f\n.integer %0 41\n.return %0\n.end\n";
    for number in 0..=5 {
        assert!(parse_function_ir(&format!("ciq-ir {number}{function}")).is_err());
        assert!(
            ciq_compiler::parse_unit_ir(&format!(
                "ciq-unit {number}\n.owner global\nciq-ir{function}"
            ))
            .is_err()
        );
    }
}
