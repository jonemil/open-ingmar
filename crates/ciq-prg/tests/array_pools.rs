use ciq_prg::data::{Fragment, Source, arrays::array_at, dictionaries::dictionary_at};

#[test]
fn observed_string_array_framing_preserves_pool_relative_references() {
    // SDK 9.2.0/fr265: ["one", "two"], relocated to DATA offset zero.
    let bytes = [
        3, 0, 0, 0, 43, 0xab, 0xcd, 0xab, 0xcd, 0, 0, 0, 12, 0, 4, b'o', b'n', b'e', 0, 0, 4, b't',
        b'w', b'o', 0, 0xda, 0x7a, 0xda, 0x7a, 0, 0, 0, 15, 5, 0, 0, 0, 2, 3, 0, 0, 0, 0, 3, 0, 0,
        0, 6, 0,
    ];
    let (array, end) = array_at(&bytes, 0).unwrap();
    assert_eq!(end, bytes.len());
    assert_eq!(array.pool.as_ref().unwrap()[1].value, "two");
    assert_eq!(array.encode().unwrap(), bytes);
    assert!(dictionary_at(&bytes, 0).is_err());
    let source = Source::from_records(&bytes).unwrap();
    assert!(matches!(source.fragments[0], Fragment::Array(_)));
    assert_eq!(
        Source::parse(&source.to_text())
            .unwrap()
            .encode()
            .unwrap()
            .0,
        bytes
    );
    for end in 0..bytes.len() {
        assert!(
            array_at(&bytes[..end], 0).is_err(),
            "accepted truncation {end}"
        );
    }
    for (at, value) in [
        (4, 42),
        (12, 13),
        (14, 0),
        (18, 0xff),
        (30, 14),
        (33, 0x0b),
        (37, 3),
        (38, 5),
        (42, 1),
        (47, 7),
        (48, 1),
    ] {
        let mut bad = bytes;
        bad[at] = value;
        assert!(array_at(&bad, 0).is_err(), "accepted corruption at {at}");
    }
    // Every accepted one-byte mutation must still round-trip exactly.
    for at in 0..bytes.len() {
        let mut changed = bytes;
        changed[at] ^= 0x80;
        if let Ok((array, end)) = array_at(&changed, 0) {
            assert_eq!(array.encode().unwrap(), changed[..end]);
        }
    }
}

#[test]
fn arrays_retain_shared_unused_duplicate_and_empty_pools_across_mixed_records() {
    let text = ".array a\n.string first \"é\"\n.string duplicate \"é\"\n.string unused \"unused\"\n.element string=&first\n.element string=&first\n.element string=&duplicate\n.element null\n.element boolean=true\n.element number=0xffffffff\n.element char=0x00110000\n.endarray\n.dictionary d\n.string key \"key\"\n.entry string=&key null\n.enddictionary\n.string first \"global\"\n.array empty\n.pool\n.endarray\n";
    let source = Source::parse(text).unwrap();
    let (bytes, strings) = source.encode().unwrap();
    assert_eq!(strings.len(), 1);
    let restored = Source::from_records(&bytes).unwrap();
    assert!(matches!(restored.fragments[0], Fragment::Array(_)));
    assert!(matches!(restored.fragments[1], Fragment::Dictionary(_)));
    assert!(matches!(restored.fragments[3], Fragment::Array(_)));
    assert_eq!(
        Source::parse(&restored.to_text())
            .unwrap()
            .encode()
            .unwrap()
            .0,
        bytes
    );
    let Fragment::Array(array) = &restored.fragments[0] else {
        panic!()
    };
    assert_eq!(array.pool.as_ref().unwrap().len(), 3);
    assert_eq!(array.values[0], array.values[1]);
    assert_ne!(array.values[0], array.values[2]);
    for bad in [
        ".array a\n.element string=&missing\n.endarray\n",
        ".array a\n.pool\n.pool\n.endarray\n",
        ".array a\n.string s \"x\"\n.string s \"y\"\n.endarray\n",
        ".array a\n.string s \"x\" at=0x00000001\n.endarray\n",
        ".array a\n.array nested\n.endarray\n.endarray\n",
    ] {
        assert!(Source::parse(bad).is_err(), "accepted {bad}");
    }
}
