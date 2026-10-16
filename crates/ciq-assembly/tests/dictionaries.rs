use ciq_assembly::{Item, Operand, Program};
use std::collections::BTreeMap;

#[test]
fn inline_requirements_preserve_local_pools_and_bind_only_dictionary_operands() {
    let source = ".dictionary dict at=0x00000020\n.string key \"x\"\n.entry string=&key number=0x00000029\n.enddictionary\nhpush &dict\nreturn\n";
    let p = Program::parse(source).unwrap();
    assert_eq!(p, Program::parse(&p.to_text()).unwrap());
    assert_eq!(p.encode().unwrap(), [0x42, 0, 0, 0, 0x20, 0x16]);
    assert!(
        p.encode_with_data_references(
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::from([("dict".into(), 0x20)])
        )
        .is_err()
    );
    for invalid in [
        source.replace(".enddictionary", ""),
        source.replace("at=0x00000020", "at=0x01000000"),
        source.replace("string=&key", "string=&missing"),
        source.replace(".entry", ".dictionary nested\n.entry"),
        format!("{source}{source}"),
    ] {
        assert!(Program::parse(&invalid).is_err());
    }
    let unplaced = source.replace(" at=0x00000020", "");
    let p = Program::parse(&unplaced).unwrap();
    assert!(p.encode().is_err());
    assert_eq!(
        p.encode_with_data_references(
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::from([("dict".into(), 0x30)])
        )
        .unwrap(),
        [0x42, 0, 0, 0, 0x30, 0x16]
    );
}

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
