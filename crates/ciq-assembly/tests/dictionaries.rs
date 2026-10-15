use ciq_assembly::{Item, Operand, Program};
use std::collections::BTreeMap;

#[test]
fn dictionary_references_have_a_separate_binding_kind_and_field_limits() {
    let p = Program::parse("hpush &dict\nreturn\n").unwrap();
    assert!(
        matches!(&p.items()[0],Item::Instruction { operands,.. } if operands == &[Operand::Dictionary("dict".into())])
    );
    assert_eq!(p, Program::parse(&p.to_text()).unwrap());
    let empty = BTreeMap::new();
    let dictionaries = BTreeMap::from([("dict".into(), 0x1234)]);
    assert_eq!(
        p.encode_with_data_references(&empty, &empty, &empty, &dictionaries)
            .unwrap(),
        [0x42, 0, 0, 0x12, 0x34, 0x16]
    );
    assert!(
        p.encode_with_literals(&empty, &empty, &dictionaries)
            .is_err()
    );
    assert!(
        p.encode_with_data_references(
            &empty,
            &empty,
            &empty,
            &BTreeMap::from([("dict".into(), 0x1000000)])
        )
        .is_err()
    );
    assert!(
        Program::new(vec![Item::Instruction {
            opcode: ciq_mc::Opcode::Apush,
            operands: vec![Operand::Dictionary("dict".into())]
        }])
        .is_err()
    );
    assert_eq!(
        Program::parse("hpush 0x00001234\nreturn\n")
            .unwrap()
            .encode()
            .unwrap(),
        [0x42, 0, 0, 0x12, 0x34, 0x16]
    );
}
