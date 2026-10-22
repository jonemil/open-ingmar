use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources_with_api, parse_function_ir, parse_unit_ir_with_api,
};

#[test]
fn number_bits_preserve_complete_pipelines() {
    let ctx = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(2, 1, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-number-bits/app.mc"),
            include_str!("../../../examples/compiler-number-bits/bits.mc"),
        ],
        ctx,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), ctx).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [55; 16],
        entry: "MinimalConsoleApp.onStart".into(),
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
}

#[test]
fn binary_bit_opcodes_and_saved_ir_are_symmetric() {
    for (source_op, directive, opcode) in [
        ("&", "bit-and", 0x07),
        ("|", "bit-or", 0x08),
        ("^", "bit-xor", 0x0c),
        ("<<", "shift-left", 0x0a),
        (">>", "shift-right", 0x0b),
    ] {
        let f =
            compile_function(&format!("function f(a, b) {{ return a {source_op} b; }}")).unwrap();
        let bytes = f.lower().encode().unwrap();
        assert_eq!(bytes, [0x35, 3, 0x12, 1, 0x12, 2, opcode, 0x16]);
        let mut encoded = Vec::new();
        for entry in ciq_mc::decode(&bytes).unwrap() {
            entry.instruction.encode_into(&mut encoded);
        }
        assert_eq!(bytes, encoded);
        let text = f.to_ir_text();
        assert!(text.contains(&format!(".{directive} %2 %0 %1")));
        assert_eq!(f, parse_function_ir(&text).unwrap());
        for replacement in [
            format!(".{directive} %2 %0"),
            format!(".{directive} %2 %0 %99"),
            format!(".{directive} %2 %0 %0"),
        ] {
            assert!(
                parse_function_ir(&text.replace(&format!(".{directive} %2 %0 %1"), &replacement))
                    .is_err()
            );
        }
        assert!(text.contains("parameters=2"));
        assert!(
            parse_function_ir(&text.replace(
                "parameters=2",
                "parameters=2 parameter-types=integer:boolean"
            ))
            .is_err()
        );
    }
    let f = compile_function("function f(a) { return ~a; }").unwrap();
    assert_eq!(f.lower().encode().unwrap(), [0x35, 2, 0x12, 1, 0x2d, 0x16]);
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
}

#[test]
fn monkey_c_precedence_and_left_associativity_match_sdk_specimens() {
    for (expr, body) in [
        ("a + b << c", vec![0x12, 1, 0x12, 2, 0x12, 3, 0x0a, 0x03]),
        ("a << b * c", vec![0x12, 1, 0x12, 2, 0x0a, 0x12, 3, 0x05]),
        ("a | b & c", vec![0x12, 1, 0x12, 2, 0x12, 3, 0x07, 0x08]),
        ("a | b ^ c", vec![0x12, 1, 0x12, 2, 0x08, 0x12, 3, 0x0c]),
        ("a ^ b + c", vec![0x12, 1, 0x12, 2, 0x0c, 0x12, 3, 0x03]),
        ("a & b * c", vec![0x12, 1, 0x12, 2, 0x07, 0x12, 3, 0x05]),
        ("a >> b >> c", vec![0x12, 1, 0x12, 2, 0x0b, 0x12, 3, 0x0b]),
        ("(a + b) << c", vec![0x12, 1, 0x12, 2, 0x03, 0x12, 3, 0x0a]),
    ] {
        let f = compile_function(&format!("function f(a, b, c) {{ return {expr}; }}")).unwrap();
        let mut expected = vec![0x35, 4];
        expected.extend(body);
        expected.push(0x16);
        assert_eq!(f.lower().encode().unwrap(), expected, "{expr}");
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    let f = compile_function("function f(a, b, c) as Toybox.Lang.Boolean { return a | b == c; }")
        .unwrap();
    assert_eq!(
        f.lower().encode().unwrap(),
        [0x35, 4, 0x12, 1, 0x12, 2, 0x08, 0x12, 3, 0x1a, 0x16]
    );
}

#[test]
fn bitwise_operators_require_nonnull_numbers_and_do_not_clamp_counts() {
    for op in ["&", "|", "^", "<<", ">>"] {
        for bad in ["true", "'a'", "\"x\"", "null", "[1]"] {
            for expr in [format!("1 {op} {bad}"), format!("{bad} {op} 1")] {
                assert!(
                    compile_function(&format!("function f() {{ return {expr}; }}")).is_err(),
                    "{expr}"
                );
            }
        }
        assert!(
            compile_function(&format!(
                "function f(n as Toybox.Lang.Number or Null) {{ return n {op} 1; }}"
            ))
            .is_err()
        );
        let f = compile_function(&format!("function f(n as Toybox.Lang.Number or Null) {{ if (n != null) {{ return n {op} 1; }} return 0; }}")).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    for bad in ["true", "'a'", "\"x\"", "null", "[1]"] {
        assert!(compile_function(&format!("function f() {{ return ~{bad}; }}")).is_err());
    }
    for count in ["-1", "0", "31", "32", "33", "2147483647", "-2147483648"] {
        for op in ["<<", ">>"] {
            let f =
                compile_function(&format!("function f(a) {{ return a {op} {count}; }}")).unwrap();
            assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        }
    }
    for source in [
        "function f() { return 1 >>> 2; }",
        "function f() { var n = 1; n <<= true; return n; }",
        "function f() { var n = 1; n |= \"x\"; return n; }",
    ] {
        assert!(compile_function(source).is_err());
    }
}

#[test]
fn complement_and_bitwise_parsing_retain_existing_budgets() {
    compile_function(&format!("function f(a) {{ return {}a; }}", "~".repeat(64))).unwrap();
    assert!(
        compile_function(&format!("function f(a) {{ return {}a; }}", "~".repeat(65)))
            .unwrap_err()
            .message
            .contains("nesting")
    );
    let expr = std::iter::repeat_n("1", 500)
        .collect::<Vec<_>>()
        .join(" | ");
    compile_function(&format!("function f() {{ return {expr}; }}")).unwrap();
}
