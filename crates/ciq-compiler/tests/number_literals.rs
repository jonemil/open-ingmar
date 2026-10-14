use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources_with_api, parse_ir, parse_unit_ir_with_api,
};

#[test]
fn signed_number_literals_choose_exact_smallest_immediate_encodings() {
    for (value, bytes) in [
        (0, vec![0x37]),
        (127, vec![0x38, 0x7f]),
        (-128, vec![0x38, 0x80]),
        (128, vec![0x39, 0, 0x80]),
        (-129, vec![0x39, 0xff, 0x7f]),
        (32767, vec![0x39, 0x7f, 0xff]),
        (-32768, vec![0x39, 0x80, 0]),
        (32768, vec![0x3a, 0, 0x80, 0]),
        (-32769, vec![0x3a, 0xff, 0x7f, 0xff]),
        (8388607, vec![0x3a, 0x7f, 0xff, 0xff]),
        (-8388608, vec![0x3a, 0x80, 0, 0]),
        (8388608, vec![0x25, 0, 0x80, 0, 0]),
        (-8388609, vec![0x25, 0xff, 0x7f, 0xff, 0xff]),
        (i32::MAX, vec![0x25, 0x7f, 0xff, 0xff, 0xff]),
        (i32::MIN, vec![0x25, 0x80, 0, 0, 0]),
    ] {
        let source = format!("function f() {{ return {value}; }}");
        let f = compile(&source).unwrap();
        assert!(
            matches!(f.operations()[0].operation, ciq_compiler::Operation::Integer { value: actual, .. } if actual == value)
        );
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        let encoded = f.lower().encode().unwrap();
        assert_eq!(&encoded[2..encoded.len() - 1], bytes);
        let decoded = ciq_mc::decode(&bytes).unwrap();
        let mut reencoded = Vec::new();
        decoded[0].instruction.encode_into(&mut reencoded);
        assert_eq!(bytes, reencoded);
        if bytes.len() > 1 {
            assert!(ciq_mc::decode(&bytes[..bytes.len() - 1]).is_err());
        }
    }
    for source in [
        "function f() { return -0; }",
        "function f() { return - /* sign */ 128; }",
        "function f() { return 1 - -2; }",
    ] {
        let f = compile(source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    let source = "function f() { return -2147483648; }";
    let f = compile(source).unwrap();
    let span = f.operations()[0].span.unwrap();
    assert_eq!(&source[span.start..span.end], "-2147483648");
}

#[test]
fn number_literals_preserve_complete_unit_and_assembly_pipelines() {
    let context = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-number-literals/app.mc"),
            include_str!("../../../examples/compiler-number-literals/numbers.mc"),
        ],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [32; 16],
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
    let bits = ciq_assembly::Program::parse(
        "ipush1 0xff\nipush2 0x8000\nipush3 0x800000\nipush4 0x80000000",
    )
    .unwrap()
    .encode()
    .unwrap();
    assert_eq!(
        bits,
        [
            0x38, 0xff, 0x39, 0x80, 0, 0x3a, 0x80, 0, 0, 0x25, 0x80, 0, 0, 0
        ]
    );
}

#[test]
fn number_literal_bounds_and_unary_scope_are_enforced_in_source_and_ir() {
    for value in [
        "2147483648",
        "-2147483649",
        "999999999999999999999999",
        "+2147483648",
        "0xff",
        "1_000",
        "1l",
    ] {
        assert!(
            compile(&format!("function f() {{ return {value}; }}")).is_err(),
            "{value}"
        );
        assert!(
            parse_ir(&format!(
                "ciq-ir\n.function f\n.integer %0 {value}\n.return %0\n.end\n"
            ))
            .is_err(),
            "{value}"
        );
    }
    for source in [
        "function f(n) { return ++n; }",
        "function f() { return +true; }",
        "function f() { return -1.abs(); }",
        "function f() { return 1--2; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    compile("function f() { return (-1).abs(); }").unwrap();
    assert!(parse_ir("ciq-ir\n.function f\n.integer %0 -\n.return %0\n.end\n").is_err());
}
