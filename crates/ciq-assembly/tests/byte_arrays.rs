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
