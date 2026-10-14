use ciq_compiler::{compile, parse_ir};
use ciq_mc::{Opcode, decode};

#[test]
fn number_complement_retains_unary_ir_and_direct_instruction_selection() {
    let source = "function f(value) { return ~value; }";
    let f = compile(source).unwrap();
    let text = f.to_ir_text();
    assert!(text.contains(".complement %1 %0"));
    assert!(!text.contains(".bit-xor") && !text.contains(".integer"));
    assert_eq!(f, parse_ir(&text).unwrap());
    assert_eq!(f.lower().encode().unwrap(), [0x35, 2, 0x12, 1, 0x2d, 0x16]);
    let operation = f
        .operations()
        .iter()
        .find(|op| matches!(op.operation, ciq_compiler::Operation::Complement { .. }))
        .unwrap();
    let span = operation.span.unwrap();
    assert_eq!(&source[span.start..span.end], "~value");
    assert!(parse_ir(&text.replace(".complement %1 %0", ".complement %1 %99")).is_err());
    assert!(parse_ir(&text.replace(".complement %1 %0", ".complement %1 %0 %0")).is_err());
    let invalid = text.replace("parameters=1", "parameters=1 parameter-types=boolean");
    assert!(parse_ir(&invalid).is_err());
    let xor = compile("function f(value) { return value ^ -1; }").unwrap();
    assert!(xor.to_ir_text().contains(".bit-xor"));
    assert!(
        decode(&xor.lower().encode().unwrap())
            .unwrap()
            .iter()
            .any(|i| i.instruction.opcode() == Opcode::Xorv)
    );
    let twice = compile("function f(value) { return ~~value; }").unwrap();
    assert_eq!(
        decode(&twice.lower().encode().unwrap())
            .unwrap()
            .iter()
            .filter(|i| i.instruction.opcode() == Opcode::Invv)
            .count(),
        2
    );
    for source in [
        "function f() { return ~true; }",
        "function f() { return ~'a'; }",
        "function f() { return ~null; }",
        "function f(value as Toybox.Lang.Number or Null) { return ~value; }",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    let guarded = compile("function f(value as Toybox.Lang.Number or Null) { if (value == null) { return 0; } return ~value; }").unwrap();
    assert_eq!(guarded, parse_ir(&guarded.to_ir_text()).unwrap());
}

#[test]
fn literal_null_comparisons_select_unary_checks_in_either_operand_order() {
    for (expression, check) in [
        ("value == null", Opcode::Isnull),
        ("null == value", Opcode::Isnull),
        ("value != null", Opcode::Isnotnull),
        ("null != value", Opcode::Isnotnull),
    ] {
        let function = compile(&format!("function f(value as Toybox.Lang.Number or Null) as Toybox.Lang.Boolean {{ return {expression}; }}")).unwrap();
        let resumed = parse_ir(&function.to_ir_text()).unwrap();
        assert_eq!(function, resumed);
        let code = function.lower().encode().unwrap();
        assert_eq!(code, resumed.lower().encode().unwrap());
        let instructions = decode(&code).unwrap();
        assert_eq!(
            instructions
                .iter()
                .map(|i| i.instruction.opcode())
                .collect::<Vec<_>>(),
            [Opcode::Argc, Opcode::Lgetv, check, Opcode::Return]
        );
        assert_eq!(instructions[1].instruction.operands(), [1]);
        assert_eq!(
            ciq_assembly::Program::parse(&function.lower().to_text())
                .unwrap()
                .encode()
                .unwrap(),
            code
        );
    }
}

#[test]
fn direct_null_checks_preserve_calls_and_nested_boolean_inversion() {
    for expression in ["\"7\".toNumber() != null", "null != \"7\".toNumber()"] {
        let f = compile(&format!(
            "function f() as Toybox.Lang.Boolean {{ return {expression}; }}"
        ))
        .unwrap();
        let assembly = f.lower();
        let opcodes = assembly
            .items()
            .iter()
            .filter_map(|item| match item {
                ciq_assembly::Item::Instruction { opcode, .. } => Some(*opcode),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            opcodes.iter().filter(|&&o| o == Opcode::Invokemz).count(),
            1
        );
        assert!(opcodes.ends_with(&[Opcode::Invokemz, Opcode::Isnotnull, Opcode::Return]));
        assert!(!opcodes.contains(&Opcode::Npush));
    }
    let f = compile("function f(value as Toybox.Lang.Number or Null) as Toybox.Lang.Boolean { return !(value == null); }").unwrap();
    assert!(
        decode(&f.lower().encode().unwrap())
            .unwrap()
            .iter()
            .map(|i| i.instruction.opcode())
            .collect::<Vec<_>>()
            .ends_with(&[Opcode::Isnull, Opcode::Invv, Opcode::Return])
    );
    let f = compile(
        "function f(value as Toybox.Lang.Number) as Toybox.Lang.Boolean { return value == 0; }",
    )
    .unwrap();
    assert!(
        decode(&f.lower().encode().unwrap())
            .unwrap()
            .iter()
            .any(|i| i.instruction.opcode() == Opcode::Eq)
    );
    let f = compile("function f() as Toybox.Lang.Boolean { return null == null; }").unwrap();
    assert!(
        decode(&f.lower().encode().unwrap())
            .unwrap()
            .iter()
            .map(|i| i.instruction.opcode())
            .collect::<Vec<_>>()
            .ends_with(&[Opcode::Npush, Opcode::Isnull, Opcode::Return])
    );
}
