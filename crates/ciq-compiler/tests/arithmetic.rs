use ciq_compiler::{compile, parse_ir};
use ciq_mc::{Opcode, decode};

fn result(bytes: &[u8]) -> i32 {
    // Independent interpreter for just this tested integer instruction subset.
    // Frame setup is checked structurally; Garmin remains the runtime oracle.
    let mut stack = Vec::new();
    let instructions = decode(bytes).unwrap();
    assert_eq!(instructions[0].instruction.opcode(), Opcode::Argc);
    assert_eq!(instructions[0].instruction.operands(), &[1]);
    for (index, located) in instructions.iter().enumerate().skip(1) {
        let instruction = &located.instruction;
        match instruction.opcode() {
            Opcode::Ipushz => stack.push(0),
            Opcode::Ipush1 => stack.push(instruction.operands()[0] as i32),
            Opcode::Addv | Opcode::Subv => {
                let right = stack.pop().unwrap();
                let left = stack.pop().unwrap();
                stack.push(if instruction.opcode() == Opcode::Addv {
                    left + right
                } else {
                    left - right
                });
            }
            Opcode::Return => {
                assert_eq!(index + 1, instructions.len());
                assert_eq!(stack.len(), 1);
                return stack.pop().unwrap();
            }
            other => panic!("unexpected opcode {other:?}"),
        }
    }
    panic!("missing return");
}

fn check(expression: &str, expected: i32) {
    let ir = compile(&format!("function probe() {{ return {expression}; }}")).unwrap();
    let model = ir.lower();
    let bytes = model.encode().unwrap();
    assert_eq!(result(&bytes), expected, "{expression}");
    let resumed = parse_ir(&ir.to_ir_text()).unwrap();
    assert_eq!(ir, resumed);
    assert_eq!(bytes, resumed.lower().encode().unwrap());
    let text_model = ciq_assembly::Program::parse(&model.to_text()).unwrap();
    assert_eq!(model, text_model);
    assert_eq!(bytes, text_model.encode().unwrap());
}

#[test]
fn grouped_arithmetic_and_left_associativity_match_in_all_pipelines() {
    for (expression, expected) in [
        ("40 + (8 - 5)", 43),
        ("10 - 3 - 2", 5),
        ("10 - (3 - 2)", 9),
        ("0 - (4 + 3)", -7),
        ("127 + 127", 254),
        ("(2 - 5) + (8 - 3)", 2),
        ("((41))", 41),
        ("40 /* + ignored */ + (8 // operands\n - 5)", 43),
    ] {
        check(expression, expected);
    }
    let model = compile("function probe() { return 40 + (8 - 5); }")
        .unwrap()
        .lower();
    assert_eq!(
        model.encode().unwrap(),
        [0x35, 1, 0x38, 40, 0x38, 8, 0x38, 5, 4, 3, 0x16]
    );
}

#[test]
fn representative_operand_pairs_check_zero_negative_and_large_results() {
    for left in [0, 1, 2, 41, 127] {
        for right in [0, 1, 2, 41, 127] {
            check(&format!("{left} + {right}"), left + right);
            check(&format!("{left} - {right}"), left - right);
        }
    }
}

#[test]
fn backend_follows_dependencies_instead_of_ir_definition_order() {
    let text = "ciq-ir\n.function probe\n.integer %10 8\n.integer %20 5\n.integer %30 40\n.subtract %40 %10 %20\n.add %50 %30 %40\n.return %50\n.end\n";
    let ir = parse_ir(text).unwrap();
    assert_eq!(result(&ir.lower().encode().unwrap()), 43);
    assert_eq!(ir.lower().encode().unwrap()[3], 40);
    assert_eq!(parse_ir(&ir.to_ir_text()).unwrap(), ir);
}

#[test]
fn invalid_expression_graphs_are_rejected_before_codegen() {
    for body in [
        ".integer %0 41\n.add %1 %0 %9\n.return %1",
        ".integer %0 41\n.add %1 %0 %0\n.return %1", // sharing needs a future policy
        ".integer %0 41\n.integer %1 42\n.return %1", // dead definition
        ".integer %0 41\n.integer %0 42\n.return %0", // duplicate
        ".add %2 %0 %1\n.integer %0 40\n.integer %1 3\n.return %2", // forward use
        ".integer %0 41\n.return %0\n.integer %1 2", // operation after return
        ".integer %0 41\n.subtract %1 %0\n.return %1", // missing operand
    ] {
        assert!(
            parse_ir(&format!("ciq-ir\n.function probe\n{body}\n.end")).is_err(),
            "{body}"
        );
    }
}

#[test]
fn malformed_source_expressions_keep_precise_diagnostics() {
    for expression in [
        "40 +",
        "(40 + 2",
        "40 + )",
        "40 ** 2",
        "40 / 2",
        "-1",
        "40 + 128",
        "40 + value",
    ] {
        assert!(
            compile(&format!("function probe() {{ return {expression}; }}")).is_err(),
            "{expression}"
        );
    }
    let error = compile("function probe() {\n    return 40 + );\n}").unwrap_err();
    assert_eq!((error.line, error.column), (2, 17));
}

#[test]
fn nesting_and_operation_limits_are_enforced() {
    check(&format!("{}1{}", "(".repeat(64), ")".repeat(64)), 1);
    let error = compile(&format!(
        "function probe() {{ return {}1{}; }}",
        "(".repeat(65),
        ")".repeat(65)
    ))
    .unwrap_err();
    assert!(error.message.contains("nesting"));
    check(&vec!["1"; 512].join(" + "), 512);
    let error = compile(&format!(
        "function probe() {{ return {}; }}",
        vec!["1"; 513].join(" + ")
    ))
    .unwrap_err();
    assert!(error.message.contains("operation limit"));
}
