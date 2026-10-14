use ciq_compiler::{
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, parse_ir, parse_unit_ir,
};
use ciq_mc::{Opcode, decode};

fn evaluate(source: &str) -> i32 {
    let function = compile(&format!("function f() {{ return {source}; }}")).unwrap();
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    let program = function.lower();
    assert_eq!(
        program,
        ciq_assembly::Program::parse(&program.to_text()).unwrap()
    );
    let code = decode(&program.encode().unwrap()).unwrap();
    let mut stack = Vec::<i32>::new();
    for located in code {
        let instruction = located.instruction;
        let bits = instruction
            .operands()
            .iter()
            .fold(0u32, |bits, byte| (bits << 8) | u32::from(*byte));
        match instruction.opcode() {
            Opcode::Argc => {}
            Opcode::Ipushz => stack.push(0),
            Opcode::Ipush1 => stack.push(bits as i8 as i32),
            Opcode::Ipush2 => stack.push(bits as i16 as i32),
            Opcode::Ipush3 => {
                let bits = bits as i32;
                stack.push((bits << 8) >> 8);
            }
            Opcode::Ipush4 => stack.push(bits as i32),
            Opcode::Addv | Opcode::Subv | Opcode::Mulv | Opcode::Divv | Opcode::Modv => {
                let right = stack.pop().unwrap();
                let left = stack.pop().unwrap();
                let result = match instruction.opcode() {
                    Opcode::Addv => left.wrapping_add(right),
                    Opcode::Subv => left.wrapping_sub(right),
                    Opcode::Mulv => left.wrapping_mul(right),
                    // The boundary MIN / -1 is intentionally excluded: SDK
                    // simulator behavior does not establish a Number result.
                    Opcode::Divv => left.checked_div(right).unwrap(),
                    Opcode::Modv => left.checked_rem(right).unwrap(),
                    _ => unreachable!(),
                };
                stack.push(result);
            }
            Opcode::Return => {}
            opcode => panic!("unexpected instruction {opcode:?}"),
        }
    }
    assert_eq!(stack.len(), 1);
    stack[0]
}

#[test]
fn number_products_preserve_precedence_associativity_signs_and_overflow() {
    for (source, expected) in [
        ("17 * 5", 85),
        ("17 / 5", 3),
        ("17 % 5", 2),
        ("-17 / 5", -3),
        ("17 / (-5)", -3),
        ("-17 / (-5)", 3),
        ("-17 % 5", -2),
        ("17 % (-5)", 2),
        ("-17 % (-5)", -2),
        ("2 + 3 * 4", 14),
        ("(2 + 3) * 4", 20),
        ("20 / 3 / 2", 3),
        ("20 / (3 / 2)", 20),
        ("20 % 6 * 3", 6),
        ("20 * 6 % 7", 1),
        ("-3 * 4", -12),
        ("2147483647 * 2", -2),
        ("-2147483648 * (-1)", i32::MIN),
        ("65536 * 65536", 0),
        ("17 / /* divisor */ 5", 3),
        ("3 /* comment */ * 4", 12),
        ("17 % // comment\n5", 2),
    ] {
        assert_eq!(evaluate(source), expected, "{source}");
    }
    for left in [-17i32, -1, 0, 1, 17] {
        for right in [-5i32, -1, 1, 5] {
            assert_eq!(evaluate(&format!("({left}) / ({right})")), left / right);
            assert_eq!(evaluate(&format!("({left}) % ({right})")), left % right);
        }
    }
    for (source, byte) in [("7 * 3", 5), ("7 / 3", 6), ("7 % 3", 9)] {
        assert_eq!(
            compile(&format!("function f() {{ return {source}; }}"))
                .unwrap()
                .lower()
                .encode()
                .unwrap(),
            [0x35, 1, 0x38, 7, 0x38, 3, byte, 0x16]
        );
    }
}

#[test]
fn product_specimen_preserves_complete_native_and_text_pipelines() {
    let unit = compile_sources(&[
        include_str!("../../../examples/compiler-number-products/app.mc"),
        include_str!("../../../examples/compiler-number-products/products.mc"),
    ])
    .unwrap();
    let text = unit.to_ir_text();
    for directive in [".multiply ", ".divide ", ".remainder "] {
        assert!(text.contains(directive));
    }
    let resumed = parse_unit_ir(&text).unwrap();
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
fn products_reject_wrong_types_and_malformed_expressions_through_saved_ir() {
    for op in ["*", "/", "%"] {
        for value in ["true", "null", "'a'", "\"5\"", "[5]", "new WatchUi.View()"] {
            for expression in [format!("5 {op} {value}"), format!("{value} {op} 5")] {
                assert!(
                    compile(&format!("function f() {{ return {expression}; }}")).is_err(),
                    "{expression}"
                );
            }
        }
    }
    for directive in ["multiply", "divide", "remainder"] {
        for literal in [
            ".boolean %1 true",
            ".char %1 0x00000061",
            ".string %1 \"5\"",
            ".null %1",
        ] {
            let text = format!(
                "ciq-ir\n.function f\n.integer %0 5\n{literal}\n.{directive} %2 %0 %1\n.return %2\n.end\n"
            );
            assert!(parse_ir(&text).is_err(), "{text}");
        }
        for operands in ["%2 %0", "%2 %0 %1 %3", "%2 %0 %9"] {
            assert!(parse_ir(&format!("ciq-ir\n.function f\n.integer %0 5\n.integer %1 2\n.{directive} {operands}\n.return %2\n.end\n")).is_err());
        }
    }
    for expression in ["5 ** 2", "5 /", "5 %", "5 *= 2", "5 /= 2", "5 %= 2"] {
        assert!(compile(&format!("function f() {{ return {expression}; }}")).is_err());
    }
    // Nullability must be proven, rather than accepting the possible Number arm.
    assert!(compile("function f(n as Toybox.Lang.String) { return n.toNumber() / 2; }").is_err());
    compile("function f(n as Toybox.Lang.String) { var value = n.toNumber(); if (value == null) { return 0; } return value / 2; }").unwrap();
}

#[test]
fn potentially_failing_operations_cannot_be_reordered_even_without_calls() {
    for first in ["divide", "remainder"] {
        let body = format!(
            ".integer %0 17\n.integer %1 0\n.{first} %2 %0 %1\n.integer %3 20\n.integer %4 0\n.divide %5 %3 %4\n"
        );
        let valid = format!("ciq-ir\n.function f\n{body}.subtract %6 %2 %5\n.return %6\n.end\n");
        parse_ir(&valid).unwrap();
        let wrong = valid.replace(".subtract %6 %2 %5", ".subtract %6 %5 %2");
        assert!(
            parse_ir(&wrong)
                .unwrap_err()
                .message
                .contains("evaluation order")
        );
    }
    // A zero divisor stays a runtime operation; the compiler does not fold it.
    for expression in ["1 / 0", "1 % 0"] {
        let function = compile(&format!("function f() {{ return {expression}; }}")).unwrap();
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    }
}
