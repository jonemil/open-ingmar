use ciq_prg::data::{Fragment, Source, dictionaries::dictionary_at};

const TEXT: &str = ".dictionary bag\n.string key \"key\" at=0x00000000\n.string text \"é\" at=0x00000006\n.entry string=&key number=0x00000029\n.entry number=0xffffffff string=&text\n.entry boolean=false char=0x00110000\n.entry null boolean=true\n.enddictionary\n.string outside \"other\"\n";

#[test]
fn structured_dictionary_preserves_pool_boundaries_order_and_scalar_bits() {
    let source = Source::parse(TEXT).unwrap();
    let (payload, strings) = source.encode().unwrap();
    let (dict, end) = dictionary_at(&payload, 0).unwrap();
    assert_eq!(dict.entries.len(), 4);
    let pool = dict.pool.as_ref().unwrap();
    assert_eq!(pool[0].at, Some(0));
    assert_eq!(pool[1].at, Some(6));
    assert_eq!(pool[1].value, "é");
    assert_eq!(dict.encode().unwrap(), payload[..end]);
    assert_eq!(strings.len(), 1);
    assert_eq!(strings["outside"], end as u32);
    assert_eq!(source.dictionary_bindings().unwrap()["bag"], 0);
    assert!(source.array_bindings().unwrap().is_empty());
    let decoded = Source::from_records(&payload).unwrap();
    assert!(matches!(decoded.fragments[0], Fragment::Dictionary(_)));
    assert_eq!(
        Source::parse(&decoded.to_text())
            .unwrap()
            .encode()
            .unwrap()
            .0,
        payload
    );
    assert_eq!(
        ciq_prg::data::record_string_offsets(&payload).unwrap(),
        [end as u32]
    );
    for text in [
        ".dictionary d\n.entry number=0x1 null\n.enddictionary\n",
        ".dictionary d\n.pool\n.enddictionary\n",
    ] {
        let payload = Source::parse(text).unwrap().encode().unwrap().0;
        let decoded = Source::from_records(&payload).unwrap();
        assert_eq!(decoded.encode().unwrap().0, payload);
        assert_eq!(
            Source::parse(&decoded.to_text())
                .unwrap()
                .encode()
                .unwrap()
                .0,
            payload
        );
    }
}

#[test]
fn truncation_bad_lengths_tags_references_and_nested_text_are_rejected() {
    let payload = Source::parse(TEXT).unwrap().encode().unwrap().0;
    let (_, end) = dictionary_at(&payload, 0).unwrap();
    let bytes = &payload[..end];
    for length in 0..bytes.len() {
        assert!(
            dictionary_at(&bytes[..length], 0).is_err(),
            "prefix {length}"
        );
    }
    for (at, value) in [
        (0, 2),
        (4, 0xff),
        (12, 0xff),
        (14, 0),
        (18, 0xff),
        (end - 1, 1),
    ] {
        let mut bad = bytes.to_vec();
        bad[at] = value;
        assert!(dictionary_at(&bad, 0).is_err(), "offset {at}");
    }
    // Invalid local String offset: encoded values may point only at pool boundaries.
    let mut bad = bytes.to_vec();
    let value_start = 5 + 8 + 11 + 8 + 5;
    bad[value_start + 4] = 1;
    assert!(dictionary_at(&bad, 0).is_err());
    for text in [
        TEXT.replace("at=0x00000006", "at=0x00000007"),
        TEXT.replace("string=&key", "string=&absent"),
        TEXT.replace(".string text", ".string key"),
        TEXT.replace(".entry null boolean=true", ".entry number=0x100000000 null"),
        TEXT.replace(".enddictionary", ".dictionary nested"),
        ".array a\n.dictionary d\n.enddictionary\n.endarray".into(),
    ] {
        assert!(Source::parse(&text).is_err(), "accepted {text}");
    }
    // Mutating any byte must either fail or retain the exact accepted representation.
    for at in 0..bytes.len() {
        let mut candidate = bytes.to_vec();
        candidate[at] ^= 0x80;
        if let Ok((dict, used)) = dictionary_at(&candidate, 0) {
            assert_eq!(dict.encode().unwrap(), candidate[..used]);
        }
    }
}
