use ciq_assembly::{Item, Operand, Program};
use ciq_mc::Opcode;
use std::collections::BTreeMap;

#[test]
fn array_operands_bind_separately_from_strings_and_follow_data_layout() {
    let source = ciq_prg::data::Source::parse(
        ".string s \"abc\"\n.array values\n.element number=0x2a\n.endarray\n",
    )
    .unwrap();
    let arrays = source.array_bindings().unwrap();
    let strings = source.string_bindings().unwrap();
    let program = Program::parse("apush &values\nnews &s\nreturn").unwrap();
    assert_eq!(
        program
            .encode_with_literals(&BTreeMap::new(), &strings, &arrays)
            .unwrap(),
        [0x40, 0, 0, 0, 7, 0x18, 0, 0, 0, 0, 0x16]
    );
    assert_eq!(program, Program::parse(&program.to_text()).unwrap());
    assert!(
        program
            .encode_with_references(&BTreeMap::new(), &arrays)
            .is_err()
    );
    for text in ["apush &s", "news &values"] {
        assert!(
            Program::parse(text)
                .unwrap()
                .encode_with_literals(&BTreeMap::new(), &strings, &arrays)
                .is_err()
        );
    }
    let moved =
        ciq_prg::data::Source::parse(".string s \"longer\"\n.array values\n.endarray\n").unwrap();
    assert_eq!(
        Program::parse("apush &values")
            .unwrap()
            .encode_with_literals(
                &BTreeMap::new(),
                &BTreeMap::new(),
                &moved.array_bindings().unwrap()
            )
            .unwrap(),
        [0x40, 0, 0, 0, 10]
    );
    assert!(ciq_mc::decode(&[0x40, 0, 0, 0]).is_err());
}

#[test]
fn branch_labels_bind_in_memory_and_text_and_follow_layout_edits() {
    let source = "start:\nbtpush\nbf code:done\njsr code:helper\ngoto code:start\nhelper:\nret\ndone:\nipush1 0x2b\nreturn\n";
    let program = Program::parse(source).unwrap();
    let bytes = program.encode().unwrap();
    assert_eq!(
        bytes,
        [
            0x3e, 0x29, 0, 7, 0x23, 0, 3, 0x19, 0xff, 0xf6, 0x17, 0x38, 43, 0x16
        ]
    );
    assert_eq!(program, Program::parse(&program.to_text()).unwrap());
    assert_eq!(
        bytes,
        Program::new(program.items().to_vec())
            .unwrap()
            .encode()
            .unwrap()
    );
    let changed = Program::parse(&source.replace("helper:\nret", "popv\nhelper:\nret")).unwrap();
    let decoded = ciq_mc::decode(&changed.encode().unwrap()).unwrap();
    assert_eq!(decoded[1].relative_target(), Some(12));
    assert_eq!(decoded[2].relative_target(), Some(11));
    assert_eq!(decoded[3].relative_target(), Some(0));
}

#[test]
fn invalid_named_branch_targets_fail_but_numeric_bits_are_preserved() {
    for source in ["argc code:x", "goto code:bad.name", "news code:x"] {
        assert!(Program::parse(source).is_err());
    }
    for source in ["goto code:missing", "goto code:end\nend:"] {
        assert!(Program::parse(source).unwrap().encode().is_err());
    }
    let typed = Program::new(vec![Item::Instruction {
        opcode: Opcode::Argc,
        operands: vec![Operand::Code("x".into())],
    }]);
    assert!(typed.is_err());
    assert_eq!(
        Program::parse("goto 0xffff").unwrap().encode().unwrap(),
        [0x19, 0xff, 0xff]
    );
}

#[test]
fn branch_displacement_endpoints_fit_and_overflow_is_rejected() {
    fn forward(padding: usize) -> Program {
        let mut items = vec![Item::Instruction {
            opcode: Opcode::Goto,
            operands: vec![Operand::Code("target".into())],
        }];
        items.extend((0..padding).map(|_| Item::Instruction {
            opcode: Opcode::Popv,
            operands: vec![],
        }));
        items.push(Item::Label("target".into()));
        items.push(Item::Instruction {
            opcode: Opcode::Return,
            operands: vec![],
        });
        Program::new(items).unwrap()
    }
    fn backward(padding: usize) -> Program {
        let mut items = vec![
            Item::Label("target".into()),
            Item::Instruction {
                opcode: Opcode::Return,
                operands: vec![],
            },
        ];
        items.extend((0..padding).map(|_| Item::Instruction {
            opcode: Opcode::Popv,
            operands: vec![],
        }));
        items.push(Item::Instruction {
            opcode: Opcode::Goto,
            operands: vec![Operand::Code("target".into())],
        });
        Program::new(items).unwrap()
    }
    assert_eq!(&forward(32767).encode().unwrap()[..3], &[0x19, 0x7f, 0xff]);
    assert!(forward(32768).encode().is_err());
    let bytes = backward(32764).encode().unwrap();
    assert_eq!(&bytes[bytes.len() - 3..], &[0x19, 0x80, 0]);
    assert!(backward(32765).encode().is_err());
}

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
