use ciq_assembly::{Item, Operand, Program};
use ciq_mc::Opcode;
use std::collections::BTreeMap;

#[test]
fn byte_array_loads_have_a_separate_namespace_and_checked_data_offsets() {
    let p = Program::parse("bpush &bytes\nreturn\n").unwrap();
    assert_eq!(p, Program::parse(&p.to_text()).unwrap());
    assert!(matches!(&p.items()[0], Item::Instruction {operands, ..}
        if operands == &[Operand::ByteArray("bytes".into())]));
    let empty = BTreeMap::new();
    let bytes = BTreeMap::from([("bytes".into(), 0x1234)]);
    assert_eq!(
        p.encode_with_byte_arrays(&empty, &empty, &empty, &empty, &bytes)
            .unwrap(),
        [0x41, 0, 0, 0x12, 0x34, 0x16]
    );
    assert!(
        p.encode_with_data_references(&bytes, &bytes, &bytes, &bytes)
            .is_err()
    );
    let high = BTreeMap::from([("bytes".into(), 0x1000000)]);
    assert!(
        p.encode_with_byte_arrays(&empty, &empty, &empty, &empty, &high)
            .is_err()
    );
    for opcode in [Opcode::News, Opcode::Apush, Opcode::Hpush] {
        assert!(
            Program::new(vec![Item::Instruction {
                opcode,
                operands: vec![Operand::ByteArray("bytes".into())]
            }])
            .is_err()
        );
    }
    for operand in [
        Operand::String("s".into()),
        Operand::Array("a".into()),
        Operand::Dictionary("d".into()),
        Operand::Symbol("s".into()),
    ] {
        assert!(
            Program::new(vec![Item::Instruction {
                opcode: Opcode::Bpush,
                operands: vec![operand]
            }])
            .is_err()
        );
    }
    let p = Program::parse("ipush1 0x03\nnewba\nreturn").unwrap();
    assert_eq!(p.encode().unwrap(), [0x38, 3, 0x36, 0x16]);
    assert!(Program::parse("newba 0x00").is_err());
    assert!(Program::parse("bpush 0x100000000").is_err());
}

#[test]
fn inline_byte_requirements_share_validation_with_data_and_native_items() {
    let text = ".bytearray bytes at=0x20\n.bytes 00ff\n.endbytearray\nbpush &bytes\nreturn\n";
    let p = Program::parse(text).unwrap();
    assert_eq!(p, Program::parse(&p.to_text()).unwrap());
    assert_eq!(p.encode().unwrap(), [0x41, 0, 0, 0, 0x20, 0x16]);
    for bad in [
        text.replace(".endbytearray", ""),
        text.replace("at=0x20", "at=0x1000000"),
        text.replace("00ff", "00f"),
        format!("{text}{text}"),
        text.replace(".bytes 00ff", ".array nested\n.endarray"),
    ] {
        assert!(Program::parse(&bad).is_err());
    }
    let empty = BTreeMap::new();
    assert!(
        p.encode_with_byte_arrays(
            &empty,
            &empty,
            &empty,
            &empty,
            &BTreeMap::from([("bytes".into(), 0x20)])
        )
        .is_err()
    );
    let unplaced = Program::parse(&text.replace(" at=0x20", "")).unwrap();
    assert!(unplaced.encode().is_err());
    assert_eq!(
        unplaced
            .encode_with_byte_arrays(
                &empty,
                &empty,
                &empty,
                &empty,
                &BTreeMap::from([("bytes".into(), 0x30)])
            )
            .unwrap(),
        [0x41, 0, 0, 0, 0x30, 0x16]
    );
}
