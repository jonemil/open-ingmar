use ciq_package::{Kind, Limits, Package};
use std::{
    fs,
    io::{Cursor, Write},
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
const BARREL: &[u8] = br#"<?xml version="1.0"?><iq:manifest xmlns:iq="http://www.garmin.com/xml/connectiq" version="3"><iq:barrel id="test-library" module="Numbers" version="1.0.0" unknown="retained"/></iq:manifest>"#;
const IQ: &[u8] = br#"<manifest xmlns="http://www.garmin.com/xml/connectiq" version="3"><application id="test-app"><products><product filename="part-a/app.prg" partNumber="part-a" custom="retained"/><product filename="part-b/app.prg" partNumber="part-b"/></products><extension unknown="yes"/></application></manifest>"#;
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "ciq-package-test-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn output(&self) -> PathBuf {
        self.0.join("output")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn barrel_entries() -> Vec<(&'static str, &'static [u8])> {
    vec![
        ("manifest.xml", BARREL),
        ("content/manifest.xml", BARREL),
        (
            "barrel.jungle",
            b"project.manifest = content/manifest.xml\n",
        ),
        ("content/source/Numbers.mc", b"module Numbers {}\n"),
        ("unknown.bin", b"\x00\xffraw"),
    ]
}
fn iq_entries() -> Vec<(&'static str, &'static [u8])> {
    vec![
        ("manifest.xml", IQ),
        ("part-a/app.prg", b"opaque-prg-a"),
        ("part-b/app.prg", b"opaque-prg-b"),
        ("unknown.bin", b"untouched"),
    ]
}
fn word(out: &mut Vec<u8>, n: u16) {
    out.extend(n.to_le_bytes());
}
fn dword(out: &mut Vec<u8>, n: u32) {
    out.extend(n.to_le_bytes());
}
// Independent fixture encoder: classic ZIP with stored members and explicit CRCs.
fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut directory = Vec::new();
    for (name, data) in entries {
        let offset = out.len() as u32;
        let crc = crc32fast::hash(data);
        out.extend(b"PK\x03\x04");
        word(&mut out, 20);
        word(&mut out, 0x800);
        word(&mut out, 0);
        word(&mut out, 0);
        word(&mut out, 0);
        dword(&mut out, crc);
        dword(&mut out, data.len() as u32);
        dword(&mut out, data.len() as u32);
        word(&mut out, name.len() as u16);
        word(&mut out, 0);
        out.extend(name.as_bytes());
        out.extend(*data);
        directory.extend(b"PK\x01\x02");
        word(&mut directory, 0x0314);
        word(&mut directory, 20);
        word(&mut directory, 0x800);
        word(&mut directory, 0);
        word(&mut directory, 0);
        word(&mut directory, 0);
        dword(&mut directory, crc);
        dword(&mut directory, data.len() as u32);
        dword(&mut directory, data.len() as u32);
        word(&mut directory, name.len() as u16);
        for _ in 0..4 {
            word(&mut directory, 0);
        }
        dword(&mut directory, 0o100644 << 16);
        dword(&mut directory, offset);
        directory.extend(name.as_bytes());
    }
    let offset = out.len() as u32;
    let size = directory.len() as u32;
    out.extend(directory);
    out.extend(b"PK\x05\x06");
    word(&mut out, 0);
    word(&mut out, 0);
    word(&mut out, entries.len() as u16);
    word(&mut out, entries.len() as u16);
    dword(&mut out, size);
    dword(&mut out, offset);
    word(&mut out, 0);
    out
}
fn number(out: &mut Vec<u8>, n: u64) {
    for bytes in 0..8 {
        if n < (1u64 << (7 + 7 * bytes)) {
            let prefix = if bytes == 0 { 0 } else { 0xff << (8 - bytes) };
            out.push(prefix | (n >> (8 * bytes)) as u8);
            out.extend(&n.to_le_bytes()[..bytes]);
            return;
        }
    }
    out.push(255);
    out.extend(n.to_le_bytes());
}
// Independently authored 7z Copy fixture. One solid block, per-member CRCs,
// unencoded next header, UTF-16LE names. No SDK or external project assets.
fn seven_z(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let payload: Vec<u8> = entries
        .iter()
        .flat_map(|(_, data)| data.iter().copied())
        .collect();
    let mut header = vec![1, 4, 6, 0, 1, 9];
    number(&mut header, payload.len() as u64);
    header.extend([0, 7, 11, 1, 0, 1, 1, 0, 12]);
    number(&mut header, payload.len() as u64);
    header.extend([0, 8, 13]);
    number(&mut header, entries.len() as u64);
    header.push(9);
    for (_, data) in entries.iter().take(entries.len() - 1) {
        number(&mut header, data.len() as u64);
    }
    header.extend([10, 1]);
    for (_, data) in entries {
        dword(&mut header, crc32fast::hash(data));
    }
    header.extend([0, 0, 5]);
    number(&mut header, entries.len() as u64);
    let mut names = vec![0];
    for (name, _) in entries {
        for unit in name.encode_utf16().chain(Some(0)) {
            word(&mut names, unit);
        }
    }
    header.push(17);
    number(&mut header, names.len() as u64);
    header.extend(names);
    header.extend([0, 0]);
    envelope(&payload, &header)
}
fn envelope(payload: &[u8], header: &[u8]) -> Vec<u8> {
    let mut start = Vec::new();
    start.extend((payload.len() as u64).to_le_bytes());
    start.extend((header.len() as u64).to_le_bytes());
    dword(&mut start, crc32fast::hash(header));
    let mut out = b"7z\xbc\xaf\x27\x1c\x00\x02".to_vec();
    dword(&mut out, crc32fast::hash(&start));
    out.extend(start);
    out.extend(payload);
    out.extend(header);
    out
}
fn open(data: Vec<u8>) -> ciq_package::Result<Package> {
    Package::read(Cursor::new(data), Limits::default())
}
fn assert_rejected(data: Vec<u8>, message: &str) {
    let error = match open(data) {
        Ok(_) => panic!("expected rejection"),
        Err(e) => e,
    };
    assert!(error.to_string().contains(message), "{error}");
}
#[test]
fn barrel_preserves_every_member_and_raw_manifest() {
    let entries = barrel_entries();
    let mut package = open(zip(&entries)).unwrap();
    assert_eq!(package.kind(), Kind::Barrel);
    assert_eq!(package.manifest_xml(), BARREL);
    assert_eq!(package.members().len(), 5);
    assert_eq!(package.manifest().identity["unknown"], "retained");
    let temp = Temp::new();
    package.unpack(temp.output()).unwrap();
    for (name, data) in entries {
        assert_eq!(fs::read(temp.output().join(name)).unwrap(), data);
    }
    assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 1);
}
#[test]
fn solid_iq_has_multiple_products_and_preserves_unknown_members() {
    let entries = iq_entries();
    let mut package = open(seven_z(&entries)).unwrap();
    assert_eq!(package.kind(), Kind::Iq);
    assert_eq!(package.manifest_xml(), IQ);
    assert_eq!(package.manifest().products.len(), 2);
    assert_eq!(
        package.manifest().products[0].attributes["custom"],
        "retained"
    );
    let temp = Temp::new();
    package.unpack(temp.output()).unwrap();
    for (name, data) in entries {
        assert_eq!(fs::read(temp.output().join(name)).unwrap(), data);
    }
}
#[test]
fn deflated_barrel_extracts() {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in barrel_entries() {
        writer
            .start_file(
                name,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(data).unwrap();
    }
    let mut package = open(writer.finish().unwrap().into_inner()).unwrap();
    let temp = Temp::new();
    package.unpack(temp.output()).unwrap();
    assert_eq!(
        fs::read(temp.output().join("unknown.bin")).unwrap(),
        b"\x00\xffraw"
    );
}
#[test]
fn paths_are_validated_before_output() {
    for name in [
        "../outside",
        "/absolute",
        "C:/drive",
        "dir\\..\\escape",
        "a/../b",
        "a//b",
        "./b",
        "NUL.txt",
        "con",
        "a. ",
        "bad\0name",
        "bad\nname",
    ] {
        let mut entries = barrel_entries();
        entries.push((name, b"bad"));
        assert_rejected(zip(&entries), "path");
        let mut entries = iq_entries();
        entries.push((name, b"bad"));
        assert!(open(seven_z(&entries)).is_err());
    }
}
#[test]
fn duplicates_case_aliases_and_parent_conflicts_are_rejected() {
    for names in [
        ["unknown.bin", "unknown.bin"],
        ["A/file", "a/other"],
        ["file", "file/child"],
        ["FILE", "file"],
        ["a/child", "a"],
    ] {
        let mut entries = barrel_entries();
        for name in names {
            entries.push((name, b"bad"));
        }
        assert!(open(zip(&entries)).is_err());
        let mut entries = iq_entries();
        for name in names {
            entries.push((name, b"bad"));
        }
        assert!(open(seven_z(&entries)).is_err());
    }
}
#[test]
fn links_special_entries_encryption_and_codecs_are_rejected() {
    for mode in [0o120777u32, 0o060600, 0o010600] {
        let mut data = zip(&barrel_entries());
        let central = data.windows(4).position(|b| b == b"PK\x01\x02").unwrap();
        data[central + 38..central + 42].copy_from_slice(&(mode << 16u32).to_le_bytes());
        assert_rejected(data, "links");
    }
    let mut encrypted = zip(&barrel_entries());
    let central = encrypted
        .windows(4)
        .position(|b| b == b"PK\x01\x02")
        .unwrap();
    encrypted[central + 8] |= 1;
    assert_rejected(encrypted, "encrypt");
    let mut codec = zip(&barrel_entries());
    let central = codec.windows(4).position(|b| b == b"PK\x01\x02").unwrap();
    codec[central + 10..central + 12].copy_from_slice(&99u16.to_le_bytes());
    assert!(open(codec).is_err());
    assert_rejected(envelope(&[], &[23]), "encoded");
    let mut header = vec![
        1, 4, 6, 0, 1, 9, 0, 0, 7, 11, 1, 0, 1, 1, 0x21, 12, 0, 0, 0, 5, 0, 0, 0,
    ];
    // Replace Copy method with LZMA2 without its required property: rejected before backend.
    assert_rejected(envelope(&[], &header), "coder");
    header[15] = 0x6;
    assert!(open(envelope(&[], &header)).is_err());
}
#[test]
fn limits_are_enforced_before_writing() {
    let data = zip(&barrel_entries());
    for limits in [
        Limits {
            archive_bytes: 10,
            ..Limits::default()
        },
        Limits {
            header_bytes: 20,
            ..Limits::default()
        },
        Limits {
            entries: 2,
            ..Limits::default()
        },
        Limits {
            member_bytes: 50,
            ..Limits::default()
        },
        Limits {
            total_bytes: 300,
            ..Limits::default()
        },
        Limits {
            manifest_bytes: 20,
            ..Limits::default()
        },
        Limits {
            path_bytes: 4,
            ..Limits::default()
        },
    ] {
        assert!(Package::read(Cursor::new(&data), limits).is_err());
    }
    assert!(
        Package::read(
            Cursor::new(seven_z(&iq_entries())),
            Limits {
                entries: 2,
                ..Limits::default()
            }
        )
        .is_err()
    );
}
#[test]
fn corrupt_payloads_clean_up_owned_output() {
    for mut data in [zip(&barrel_entries()), seven_z(&iq_entries())] {
        let needle: &[u8] = if data.starts_with(b"PK") {
            b"\x00\xffraw"
        } else {
            b"untouched"
        };
        let at = data
            .windows(needle.len())
            .position(|b| b == needle)
            .unwrap();
        data[at] ^= 1;
        let mut package = open(data).unwrap();
        let temp = Temp::new();
        assert!(package.unpack(temp.output()).is_err());
        assert!(!temp.output().exists());
        assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 0);
    }
}
#[test]
fn existing_destinations_are_untouched() {
    let mut package = open(zip(&barrel_entries())).unwrap();
    let temp = Temp::new();
    fs::create_dir(temp.output()).unwrap();
    fs::write(temp.output().join("keep"), b"user data").unwrap();
    assert!(package.unpack(temp.output()).is_err());
    assert_eq!(fs::read(temp.output().join("keep")).unwrap(), b"user data");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(temp.output(), temp.0.join("linked")).unwrap();
        assert!(package.unpack(temp.0.join("linked")).is_err());
        assert_eq!(fs::read(temp.output().join("keep")).unwrap(), b"user data");
    }
}
#[test]
fn bad_manifests_and_missing_products_fail() {
    for xml in [b"<bad/>".as_slice(),b"<!DOCTYPE manifest><bad/>",b"<manifest xmlns='http://www.garmin.com/xml/connectiq' version='3'><application id='a'><products><product filename='../escape' partNumber='p'/></products></application></manifest>",b"<manifest xmlns='http://www.garmin.com/xml/connectiq' version='3'><application id='a'><products><product filename='missing.prg' partNumber='p'/></products></application></manifest>",b"<manifest xmlns='http://www.garmin.com/xml/connectiq' version='3'><application id='a'><products><product filename='part-a/app.prg' filename='duplicate' partNumber='p'/></products></application></manifest>",b"<manifest xmlns='http://www.garmin.com/xml/connectiq' version='3'><application id='a'><products xmlns='foreign'><product xmlns='http://www.garmin.com/xml/connectiq' filename='part-a/app.prg' partNumber='p'/></products></application></manifest>"] {
        let mut entries = iq_entries(); entries[0].1 = xml; assert!(open(seven_z(&entries)).is_err());
    }
    let mut entries = barrel_entries();
    entries[1].1 = b"bad xml";
    assert!(open(zip(&entries)).is_err());
    let mut entries = barrel_entries();
    entries.retain(|(name, _)| *name != "barrel.jungle");
    assert_rejected(zip(&entries), "missing");
}
#[test]
fn truncated_archives_never_panic() {
    for data in [zip(&barrel_entries()), seven_z(&iq_entries())] {
        for len in 0..data.len() {
            assert!(open(data[..len].to_vec()).is_err(), "prefix {len}");
        }
    }
}
#[test]
fn header_count_forgery_and_crc_damage_fail() {
    let mut data = zip(&barrel_entries());
    let end = data.len() - 22;
    data[end + 8..end + 12].fill(0);
    assert_rejected(data, "count mismatch");
    let mut data = seven_z(&iq_entries());
    data[8] ^= 1;
    assert!(open(data).is_err());
    let data = seven_z(&iq_entries());
    let header_at = 32 + u64::from_le_bytes(data[12..20].try_into().unwrap()) as usize;
    let mut header = data[header_at..].to_vec();
    let fileinfo = header.windows(2).position(|b| b == [5, 4]).unwrap();
    header[fileinfo + 1] = 1;
    assert!(open(envelope(&data[32..header_at], &header)).is_err());
}
#[test]
fn cli_lists_unpacks_and_reports_verification_boundary() {
    let temp = Temp::new();
    let input = temp.0.join("library.barrel");
    fs::write(&input, zip(&barrel_entries())).unwrap();
    let list = Command::new(env!("CARGO_BIN_EXE_ciq-package"))
        .arg("list")
        .arg(&input)
        .output()
        .unwrap();
    assert!(list.status.success());
    let text = String::from_utf8(list.stdout).unwrap();
    assert!(text.contains("unknown.bin"));
    assert!(text.contains("Signatures were not verified"));
    let unpack = Command::new(env!("CARGO_BIN_EXE_ciq-package"))
        .arg("unpack")
        .arg(&input)
        .arg(temp.output())
        .output()
        .unwrap();
    assert!(unpack.status.success(), "{:?}", unpack);
    assert_eq!(
        fs::read(temp.output().join("manifest.xml")).unwrap(),
        BARREL
    );
    let again = Command::new(env!("CARGO_BIN_EXE_ciq-package"))
        .arg("unpack")
        .arg(&input)
        .arg(temp.output())
        .output()
        .unwrap();
    assert!(!again.status.success());
}

#[test]
fn malformed_next_headers_are_bounded_and_do_not_panic() {
    let data = seven_z(&iq_entries());
    let at = 32 + u64::from_le_bytes(data[12..20].try_into().unwrap()) as usize;
    for offset in 0..data.len() - at {
        for value in [0, 1, 0x7f, 0x80, 0xff] {
            let mut header = data[at..].to_vec();
            header[offset] = value;
            let _ = open(envelope(&data[32..at], &header));
        }
    }
    let mut header = data[at..].to_vec();
    // A terminated filename table followed by another unterminated UTF-16 name
    // must not be passed to a backend which indexes files as it parses names.
    let property = header.iter().rposition(|&b| b == 17).unwrap();
    let size_byte = property + 1;
    // This fixture uses a two-byte 7z size, with low byte immediately after prefix.
    header[size_byte + 1] += 2;
    header.splice(header.len() - 2..header.len() - 2, [b'x', 0]);
    assert!(open(envelope(&data[32..at], &header)).is_err());
}

#[test]
fn seven_z_links_reparse_points_and_large_dictionaries_fail_preflight() {
    let data = seven_z(&iq_entries());
    let at = 32 + u64::from_le_bytes(data[12..20].try_into().unwrap()) as usize;
    for attributes in [0o120777u32 << 16 | 0x8000, 0x400] {
        let mut header = data[at..].to_vec();
        let mut property = vec![21, 18, 1, 0];
        for i in 0..4 {
            dword(&mut property, if i == 3 { attributes } else { 0 });
        }
        header.splice(header.len() - 2..header.len() - 2, property);
        assert!(open(envelope(&data[32..at], &header)).is_err());
    }
    let header = [1, 4, 6, 0, 1, 9, 0, 0, 7, 11, 1, 0, 1, 0x21, 0x21, 1, 40];
    assert_rejected(envelope(&[], &header), "dictionary");
    let header = [1, 4, 6, 0, 1, 9, 0, 0, 7, 11, 1, 0, 1, 4, 6, 0xf1, 7, 1];
    assert_rejected(envelope(&[], &header), "encrypted");
}
#[test]
fn directory_entries_and_empty_files_are_preserved_without_permissions() {
    let mut entries = barrel_entries();
    entries.extend([
        ("content/", b"".as_slice()),
        ("content/source/", b"".as_slice()),
        ("empty.bin", b"".as_slice()),
    ]);
    let mut package = open(zip(&entries)).unwrap();
    let temp = Temp::new();
    package.unpack(temp.output()).unwrap();
    assert!(temp.output().join("content/source").is_dir());
    assert_eq!(fs::read(temp.output().join("empty.bin")).unwrap(), b"");
}

#[test]
fn namespace_and_manifest_complexity_limits_are_checked() {
    let namespaces = (0..129)
        .map(|i| format!(" xmlns:n{i}='urn:{i}'"))
        .collect::<String>();
    let xml = format!(
        "<manifest xmlns='http://www.garmin.com/xml/connectiq' version='3'{namespaces}><barrel id='x' module='X' version='1'/></manifest>"
    );
    let mut entries = barrel_entries();
    entries[0].1 = xml.as_bytes();
    assert!(open(zip(&entries)).is_err());
    let xml = format!(
        "<manifest xmlns='http://www.garmin.com/xml/connectiq' version='3'><barrel id='x' module='X' version='1'/>{}{}</manifest>",
        "<unknown>".repeat(129),
        "</unknown>".repeat(129)
    );
    entries[0].1 = xml.as_bytes();
    assert!(open(zip(&entries)).is_err());
    let xml = b"<manifest xmlns='http://www.garmin.com/xml/connectiq' version='3'><barrel id='x' module='X' version='1'/><unknown>&undefined;</unknown></manifest>";
    entries[0].1 = xml;
    assert!(open(zip(&entries)).is_err());
}
#[test]
fn iq_products_may_share_one_ordinary_member() {
    let xml = b"<manifest xmlns='http://www.garmin.com/xml/connectiq' version='3'><application id='x'><products><product filename='app.prg' partNumber='one'/><product filename='app.prg' partNumber='two'/></products></application></manifest>";
    let mut package = open(seven_z(&[("manifest.xml", xml), ("app.prg", b"opaque")])).unwrap();
    assert_eq!(package.manifest().products.len(), 2);
    let temp = Temp::new();
    package.unpack(temp.output()).unwrap();
    assert_eq!(fs::read(temp.output().join("app.prg")).unwrap(), b"opaque");
}
