use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources_with_api, parse_function_ir, parse_unit_ir_with_api,
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
        let source = format!("function f() as Toybox.Lang.Number {{ return {value}; }}");
        let f = compile_function(&source).unwrap();
        assert!(
            matches!(f.operations()[0].operation, ciq_compiler::Operation::Integer { value: actual, .. } if actual == value)
        );
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
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
        "function f() as Toybox.Lang.Number { return -0; }",
        "function f() as Toybox.Lang.Number { return - /* sign */ 128; }",
        "function f() as Toybox.Lang.Number { return 1 - -2; }",
    ] {
        let f = compile_function(source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    let source = "function f() as Toybox.Lang.Number { return -2147483648; }";
    let f = compile_function(source).unwrap();
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
        "1_000",
        "1l",
    ] {
        assert!(
            compile_function(&format!(
                "function f() as Toybox.Lang.Number {{ return {value}; }}"
            ))
            .is_err(),
            "{value}"
        );
        assert!(
            parse_function_ir(&format!(
                "ciq-ir\n.function f\n.integer %0 {value}\n.return %0\n.end\n"
            ))
            .is_err(),
            "{value}"
        );
    }
    for source in [
        "function f(n as Toybox.Lang.Number) as Toybox.Lang.Number { return ++n; }",
        "function f() as Toybox.Lang.Number { return +true; }",
        "function f() as Toybox.Lang.Number { return -1.abs(); }",
        "function f() as Toybox.Lang.Number { return 1--2; }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    compile_function("function f() as Toybox.Lang.Number { return (-1).abs(); }").unwrap();
    assert!(parse_function_ir("ciq-ir\n.function f\n.integer %0 -\n.return %0\n.end\n").is_err());
}

#[test]
fn radix_numbers_use_width_sized_bits_and_wrapping_literal_negation() {
    for (literal, value) in [
        ("0xff", 255),
        ("0xFF", 255),
        ("077", 63),
        ("00", 0),
        ("0xffffffff", -1),
        ("037777777777", -1),
        ("0x80000000", i32::MIN),
        ("020000000000", i32::MIN),
        ("-0xffffffff", 1),
        ("-037777777777", 1),
        ("-0x80000000", i32::MIN),
    ] {
        let function = compile_function(&format!(
            "function f() as Toybox.Lang.Number{{return {literal};}}"
        ))
        .unwrap();
        assert!(
            matches!(function.operations()[0].operation,ciq_compiler::Operation::Integer{value:actual,..} if actual==value),
            "{literal}"
        );
        assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
        assert_eq!(
            function.lower().encode().unwrap(),
            compile_function(&format!(
                "function f() as Toybox.Lang.Number{{return {value};}}"
            ))
            .unwrap()
            .lower()
            .encode()
            .unwrap()
        );
    }
    for literal in [
        "0x",
        "0xg",
        "0Xff",
        "08",
        "019",
        "0b1",
        "0x100000000",
        "040000000000",
        "-0x100000000",
        "1_000",
    ] {
        assert!(
            compile_function(&format!(
                "function f() as Toybox.Lang.Number{{return {literal};}}"
            ))
            .is_err(),
            "{literal}"
        );
    }
}
