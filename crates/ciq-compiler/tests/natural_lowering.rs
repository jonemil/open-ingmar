use ciq_compiler::{compile, parse_ir};
use ciq_mc::{Opcode, decode};

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
