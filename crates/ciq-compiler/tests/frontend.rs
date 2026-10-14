use ciq_compiler::{Operation, ValueId, compile};
use ciq_mc::{Opcode, decode};

#[test]
fn semantic_ir_preserves_value_identity_and_source_spans() {
    let source = "// comment\nfunction probe() {\n    return 41;\n}\n";
    let function = compile(source).unwrap();
    assert_eq!(function.name(), "probe");
    let operations = function.operations();
    assert_eq!(
        operations[0].operation,
        Operation::Integer {
            result: ValueId(0),
            value: 41
        }
    );
    assert_eq!(
        operations[1].operation,
        Operation::Return { value: ValueId(0) }
    );
    assert_eq!(
        &source[operations[0].span.unwrap().start..operations[0].span.unwrap().end],
        "41"
    );
    assert_eq!(
        &source[operations[1].span.unwrap().start..operations[1].span.unwrap().end],
        "return 41;"
    );
}

#[test]
fn every_supported_literal_assembles_and_returns_its_value() {
    // Interpret only the supported constant-return sequence, independently of
    // the frontend IR. This is an encoding check, not a full VM/runtime oracle.
    for expected in 0..=127 {
        let function = compile(&format!("function probe() {{ return {expected}; }}")).unwrap();
        let bytes = ciq_asm::assemble(&function.assembly()).unwrap();
        let mut result = None;
        let mut returned = false;
        let instructions = decode(&bytes).unwrap();
        assert_eq!(instructions[0].instruction.opcode(), Opcode::Argc);
        assert_eq!(instructions[0].instruction.operands(), &[1]);
        for instruction in &instructions[1..] {
            match instruction.instruction.opcode() {
                Opcode::Ipushz => result = Some(0),
                Opcode::Ipush1 => result = Some(instruction.instruction.operands()[0] as i8 as i32),
                Opcode::Return => {
                    assert_eq!(result, Some(expected));
                    returned = true;
                }
                other => panic!("unexpected instruction {other:?}"),
            }
        }
        assert!(returned);
        assert_eq!(
            ciq_asm::code_labels(&function.assembly()).unwrap()["probe"],
            0
        );
    }
}

#[test]
fn unsupported_programs_are_rejected_completely() {
    for source in [
        "",
        "function probe(x as Number) { return 41; }",
        "function probe() { return 2147483648; }",
        "function probe() { return 999999999999999999999999; }",
        "function probe() { return -2147483649; }",
        "function probe() { return 0x29; }",
        "function probe() { return 40 ** 1; }",
        "function probe() { return true; }",
        "function probe() { return 41; return 42; }",
        "function probe() { return 41; } function other() { return 42; }",
        "function return() { return 41; }",
        "module Left { function probe() { return 41; } }",
        "function probe() { return 41;",
        "function probe() { /* unterminated",
    ] {
        assert!(
            compile(source).is_err(),
            "accepted unsupported input {source:?}"
        );
    }
}

#[test]
fn comments_and_unicode_diagnostics_keep_correct_source_locations() {
    let function = compile(
        "/* café */ function /* name */ probe(// args\n) { return /* value */ 41; } // eof",
    )
    .unwrap();
    assert_eq!(function.name(), "probe");
    let source = "/* café */\nfunction probe() {\n  return 2147483648;\n}";
    let error = compile(source).unwrap_err();
    assert_eq!((error.line, error.column), (3, 10));
    assert_eq!(&source[error.offset..error.offset + 10], "2147483648");
    let error = compile("/* é */ function probe() { return 💡; }").unwrap_err();
    assert_eq!((error.line, error.column), (1, 35));
    assert!(error.message.contains("unsupported character"));
}
