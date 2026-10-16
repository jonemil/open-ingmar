use ciq_assembly::Program;

#[test]
fn long_push_preserves_all_eight_operand_bytes() {
    for bits in [
        0,
        1,
        0x0102030405060708,
        0x7fffffffffffffff,
        0x8000000000000000,
        u64::MAX,
    ] {
        let program = Program::parse(&format!("lpush 0x{bits:016x}\nlpushz")).unwrap();
        let code = program.encode().unwrap();
        assert_eq!(code[0], 0x31);
        assert_eq!(&code[1..9], &bits.to_be_bytes());
        assert_eq!(code[9], 0x3c);
        let decoded = ciq_mc::decode(&code).unwrap();
        let mut encoded = Vec::new();
        for instruction in decoded {
            instruction.instruction.encode_into(&mut encoded);
        }
        assert_eq!(code, encoded);
        assert_eq!(program, Program::parse(&program.to_text()).unwrap());
    }
    assert!(Program::parse("lpush 0x10000000000000000").is_err());
    assert!(Program::parse("ipush4 0x100000000").is_err());
    assert!(ciq_mc::decode(&[0x31, 0, 1, 2, 3, 4, 5, 6]).is_err());
}
