use ciq_compiler::{
    Function, Operation, ParameterType, ReturnType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources_with_api, parse_ir, parse_ir_with_api, parse_unit_ir_with_api,
};

#[test]
fn array_reads_preserve_complete_pipelines_and_postfix_order() {
    let ctx = ApiContext::new(ApiLevel(1, 3, 0), ApiLevel(1, 3, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-array-reads/app.mc"),
            include_str!("../../../examples/compiler-array-reads/arrays.mc"),
        ],
        ctx,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), ctx).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [38; 16],
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
    let source = "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>, n) as Toybox.Lang.Char { return a[n]; }";
    let f = compile(source).unwrap();
    assert_eq!(
        f.lower().encode().unwrap(),
        [0x35, 3, 0x12, 1, 0x12, 2, 0x10, 0x16]
    );
    let op = f
        .operations()
        .iter()
        .find(|op| matches!(op.operation, Operation::ArrayRead { .. }))
        .unwrap();
    let span = op.span.unwrap();
    assert_eq!(&source[span.start..span.end], "a[n]");
    let f = compile("function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Toybox.Lang.String { return a[0].toUpper().toString(); }").unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let f = compile("function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Toybox.Lang.Char { return a[(true || false).hashCode()]; }").unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
}

#[test]
fn array_reads_recheck_types_guards_and_runtime_index_obligations() {
    for source in [
        "function f() as Toybox.Lang.Char { return \"abc\"[0]; }",
        "function f() as Toybox.Lang.Char { return 1[0]; }",
        "function f() as Toybox.Lang.Char { return \"abc\".toCharArray()[true]; }",
        "function f() as Toybox.Lang.Char { return \"abc\".toCharArray()[null]; }",
        "function f() as Toybox.Lang.Char { return \"abc\".toCharArray()['A']; }",
        "function f() as Toybox.Lang.Char { return \"abc\".toCharArray()[0][0]; }",
        "function f() { var a = \"abc\".toCharArray(); a[0] = 'X'; return 0; }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Char { return \"abc\".toCharArray()[n]; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let f = compile("function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Char { if (n == null) { return 'X'; } return \"abc\".toCharArray()[n]; }").unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    for expr in [
        "\"ab\".toCharArray()[-1]",
        "\"ab\".toCharArray()[2]",
        "\"\".toCharArray()[0]",
    ] {
        compile(&format!(
            "function f() as Toybox.Lang.Char {{ return {expr}; }}"
        ))
        .unwrap();
    }
    let f = compile("function f(a as Toybox.Lang.Array<Toybox.Lang.Char>, n) as Toybox.Lang.Char { return a[n]; }").unwrap();
    assert!(
        parse_ir(
            &f.to_ir_text()
                .replace("char-array:integer", "char-array:boolean")
        )
        .is_err()
    );
    assert!(
        parse_ir(
            &f.to_ir_text()
                .replace("char-array:integer", "string:integer")
        )
        .is_err()
    );
    let mut operations = f.operations().to_vec();
    for op in &mut operations {
        if let Operation::ArrayRead { index, .. } = &mut op.operation {
            *index = ciq_compiler::ValueId(999);
        }
    }
    assert!(
        Function::with_signature(
            "bad".into(),
            vec![ParameterType::CharArray, ParameterType::Integer],
            ReturnType::Char,
            operations
        )
        .is_err()
    );
    assert!(
        parse_ir_with_api(
            &f.to_ir_text(),
            ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap()
        )
        .is_err()
    );
}

#[test]
fn bounds_sensitive_reads_preserve_order_even_without_calls_or_writes() {
    let f = compile("function f(a as Toybox.Lang.Array<Toybox.Lang.Char>, b as Toybox.Lang.Array<Toybox.Lang.Char>) as Toybox.Lang.String { return a[0] + b[0]; }").unwrap();
    let mut operations = f.operations().to_vec();
    let reads = operations
        .iter()
        .enumerate()
        .filter_map(|(i, op)| matches!(op.operation, Operation::ArrayRead { .. }).then_some(i))
        .collect::<Vec<_>>();
    // Move the second read's definitions before the first read. All operands
    // remain defined, but returning left+right would reverse the recorded reads.
    let moved = operations
        .drain(reads[1] - 2..=reads[1])
        .collect::<Vec<_>>();
    operations.splice(reads[0]..reads[0], moved);
    let error = Function::with_signature(
        "bad".into(),
        vec![ParameterType::CharArray; 2],
        ReturnType::String,
        operations,
    )
    .unwrap_err();
    assert!(
        error.message.contains("evaluation order"),
        "{}",
        error.message
    );
}

#[test]
fn indexing_and_member_calls_share_the_existing_nesting_budget() {
    let source = |n: usize| {
        let mut expression = "0".to_string();
        for _ in 0..n {
            expression = format!("a[{expression}].toNumber() - 65");
        }
        format!("function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) {{ return {expression}; }}")
    };
    compile(&source(63)).unwrap();
    assert!(
        compile(&source(64))
            .unwrap_err()
            .message
            .contains("nesting")
    );
}
