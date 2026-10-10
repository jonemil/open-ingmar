use ciq_assembly::{Item, Operand, Program};
use ciq_mc::Opcode;
use std::collections::BTreeMap;

#[test]
fn typed_references_declarations_and_layout_survive_text_roundtrip() {
    let text = "start:\ngetmv @system @println\nnews &greeting\nreturn\nend:\n.symbol system \"Toybox_System\" id=0x00800005\n.symbol println \"println\" id=0x00800067\n.string greeting \"Hello\\n\\\"there\\\"\" at=0x00000010\n";
    let model = Program::parse(text).unwrap();
    let reloaded = Program::parse(&model.to_text()).unwrap();
    assert_eq!(model, reloaded);
    assert_eq!(model.labels().unwrap(), reloaded.labels().unwrap());
    assert_eq!(model.encode().unwrap(), reloaded.encode().unwrap());
    assert_eq!(model.labels().unwrap()["end"], 15);
}

#[test]
fn memory_construction_uses_the_same_operand_and_layout_validation() {
    let instructions = [
        (Opcode::Argc, vec![Operand::Bits(256)]),
        (Opcode::Argc, vec![Operand::Symbol("x".into())]),
        (Opcode::Spush, vec![Operand::String("x".into())]),
        (Opcode::Return, vec![Operand::Bits(1)]),
        (Opcode::News, vec![Operand::String("invalid.name".into())]),
    ];
    for (opcode, operands) in instructions {
        assert!(Program::new(vec![Item::Instruction { opcode, operands }]).is_err());
    }
    assert!(Program::new(vec![Item::Label("same".into()), Item::Label("same".into())]).is_err());
    let model = Program::new(vec![Item::Instruction {
        opcode: Opcode::Getselfv,
        operands: vec![Operand::Symbol("external".into())],
    }])
    .unwrap();
    assert!(model.encode().is_err());
    let symbols = BTreeMap::from([("external".into(), 0x00800005)]);
    assert_eq!(
        model
            .encode_with_references(&symbols, &BTreeMap::new())
            .unwrap(),
        [0x43, 0, 0x80, 0, 5]
    );
    let reloaded = Program::parse(&model.to_text()).unwrap();
    assert_eq!(
        model
            .encode_with_references(&symbols, &BTreeMap::new())
            .unwrap(),
        reloaded
            .encode_with_references(&symbols, &BTreeMap::new())
            .unwrap()
    );
}
#[test]
fn unplaced_literals_require_external_data_bindings() {
    let program =
        ciq_assembly::Program::parse(".string greeting \"hello\"\nnews &greeting\nreturn").unwrap();
    assert!(program.encode().is_err());
    let invalid = std::collections::BTreeMap::from([("greeting".into(), 0x1000000)]);
    assert!(
        program
            .encode_with_references(&Default::default(), &invalid)
            .is_err()
    );
    let bindings = std::collections::BTreeMap::from([("greeting".into(), 0x123)]);
    let bytes = program
        .encode_with_references(&Default::default(), &bindings)
        .unwrap();
    assert_eq!(
        ciq_mc::decode(&bytes).unwrap()[0].instruction.operands(),
        &[0, 0, 1, 0x23]
    );
    assert!(
        ciq_assembly::Program::parse(".string greeting \"hello\"\n.string greeting \"again\"")
            .is_err()
    );
    assert_eq!(
        program.to_text(),
        ciq_assembly::Program::parse(&program.to_text())
            .unwrap()
            .to_text()
    );
}
