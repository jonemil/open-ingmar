use ciq_prg::data::{
    Fragment, Source,
    arrays::{Declaration, Value, array_at},
};

#[test]
fn numeric_tags_preserve_exact_width_bits_and_record_boundaries() {
    let values = vec![
        Value::Float(0x3fa00000),
        Value::Float(0x80000000),
        Value::Float(0x7fc12345),
        Value::Long(0x0000000100000001),
        Value::Long(0x8000000000000000),
        Value::Long(u64::MAX),
        Value::Double(0x4170000010000000),
        Value::Double(0x8000000000000000),
        Value::Double(0x7ff8000000001234),
        Value::Null,
    ];
    let a = Declaration {
        alias: "values".into(),
        at: None,
        pool: None,
        values,
    };
    let bytes = a.encode().unwrap();
    assert_eq!(&bytes[18..23], &[2, 0x3f, 0xa0, 0, 0]);
    assert_eq!(&bytes[33..42], &[0x0e, 0, 0, 0, 1, 0, 0, 0, 1]);
    assert_eq!(&bytes[60..69], &[0x0f, 0x41, 0x70, 0, 0, 0x10, 0, 0, 0]);
    let (decoded, end) = array_at(&bytes, 0).unwrap();
    assert_eq!(end, bytes.len());
    assert_eq!(decoded.values, a.values);
    assert_eq!(decoded.encode().unwrap(), bytes);
    let source = Source::parse(&a.to_text()).unwrap();
    let Fragment::Array(text) = &source.fragments[0] else {
        panic!()
    };
    assert_eq!(text.values, a.values);
    for size in 0..bytes.len() {
        assert!(array_at(&bytes[..size], 0).is_err());
    }
    for tag in [4, 6, 0xff] {
        let mut bad = bytes.clone();
        bad[18] = tag;
        assert!(array_at(&bad, 0).is_err());
    }
    let mut bad = bytes.clone();
    bad[17] += 1;
    assert!(array_at(&bad, 0).is_err());
    let mut bad = bytes.clone();
    bad[4] += 1;
    assert!(array_at(&bad, 0).is_err());
}

#[test]
fn numeric_data_text_rejects_overflow_and_invalid_hex() {
    for value in [
        "float=0x100000000",
        "long=0x10000000000000000",
        "double=0x10000000000000000",
        "long=-1",
        "double=0xgg",
        "float=1.25",
    ] {
        assert!(
            Source::parse(&format!(".array values\n.element {value}\n.endarray\n")).is_err(),
            "{value}"
        );
    }
}
