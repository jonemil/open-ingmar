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
    assert_rejected(envelope(&[], &[23]), "truncated");
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

mod verification {
    use super::*;
    use ciq_package::Check;
    use ciq_prg::{CODE, Prg, Section};
    use ciq_sign::{DeveloperKey, HashAlgorithm, PublicKey};
    use rand_core::OsRng;
    use rsa::{RsaPrivateKey, pkcs8::EncodePrivateKey};
    use std::sync::OnceLock;

    fn iq_tree(temp: &Temp, fixture: &Fixture) -> PathBuf {
        let root = temp.0.join("source");
        fs::create_dir(&root).unwrap();
        for (name, bytes) in &fixture.entries {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        root
    }
    #[test]
    fn iq_repack_preserves_members_signatures_and_is_deterministic() {
        let fixture = Fixture::new();
        let temp = Temp::new();
        let root = iq_tree(&temp, &fixture);
        fs::create_dir(root.join("empty-directory")).unwrap();
        assert!(
            ciq_package::repack_iq(&root, temp.output(), Limits::default())
                .unwrap_err()
                .to_string()
                .contains("empty directories")
        );
        assert!(!temp.output().exists());
        fs::remove_dir(root.join("empty-directory")).unwrap();
        fs::write(root.join("empty.bin"), []).unwrap();
        ciq_package::repack_iq(&root, temp.output(), Limits::default()).unwrap();
        let second = temp.0.join("second.iq");
        let result = Command::new(env!("CARGO_BIN_EXE_ciq-package"))
            .arg("repack-iq")
            .arg(&root)
            .arg(&second)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(fs::read(temp.output()).unwrap(), fs::read(second).unwrap());
        let mut package = Package::open(temp.output(), Limits::default()).unwrap();
        assert!(package.verify(Some(&keys()[0].public_key())).verified());
        let extracted = temp.0.join("extracted");
        package.unpack(&extracted).unwrap();
        for (name, bytes) in &fixture.entries {
            assert_eq!(&fs::read(extracted.join(name)).unwrap(), bytes);
        }
        assert_eq!(fs::read(extracted.join("empty.bin")).unwrap(), []);
    }
    #[test]
    fn iq_repack_rejects_stale_or_invalid_signatures_without_output() {
        let fixture = Fixture::new();
        let temp = Temp::new();
        let root = iq_tree(&temp, &fixture);
        for name in ["manifest.xml", "a/app.prg", "manifest.sig2", "dev_key.pub"] {
            let path = root.join(name);
            let original = fs::read(&path).unwrap();
            let mut changed = original.clone();
            if name == "manifest.xml" {
                changed.push(b'\n');
            } else {
                let last = changed.len() - 1;
                changed[last] ^= 1;
            }
            fs::write(&path, changed).unwrap();
            assert!(
                ciq_package::repack_iq(&root, temp.output(), Limits::default()).is_err(),
                "{name}"
            );
            assert!(!temp.output().exists());
            fs::write(path, original).unwrap();
        }
    }
    #[test]
    fn iq_repack_refuses_existing_outputs_and_obeys_limits() {
        let fixture = Fixture::new();
        let temp = Temp::new();
        let root = iq_tree(&temp, &fixture);
        for limits in [
            Limits {
                archive_bytes: 64,
                ..Limits::default()
            },
            Limits {
                entries: 2,
                ..Limits::default()
            },
            Limits {
                header_bytes: 1,
                ..Limits::default()
            },
        ] {
            assert!(ciq_package::repack_iq(&root, temp.output(), limits).is_err());
            assert!(!temp.output().exists());
        }
        fs::write(temp.output(), b"preserve").unwrap();
        assert!(ciq_package::repack_iq(&root, temp.output(), Limits::default()).is_err());
        assert_eq!(fs::read(temp.output()).unwrap(), b"preserve");
        assert!(ciq_package::repack_iq(&root, root.join("inside.iq"), Limits::default()).is_err());
    }
    fn keys() -> &'static [DeveloperKey; 2] {
        static KEYS: OnceLock<[DeveloperKey; 2]> = OnceLock::new();
        KEYS.get_or_init(|| {
            std::array::from_fn(|_| {
                let key = RsaPrivateKey::new(&mut OsRng, 4096).unwrap();
                DeveloperKey::from_pkcs8_der(key.to_pkcs8_der().unwrap().as_bytes()).unwrap()
            })
        })
    }
    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
    struct Fixture {
        entries: Vec<(String, Vec<u8>)>,
    }
    impl Fixture {
        fn new() -> Self {
            let key = &keys()[0];
            let prg = ciq_sign::resign(
                &Prg::from_sections(vec![Section::new(CODE, vec![0x38, 41, 0x16]).unwrap()]),
                key,
            )
            .unwrap();
            let fields = ciq_sign::developer_signatures(&prg).unwrap();
            let product = format!(
                "sigFormat='rsa-4096' sig='{}' sig2='{}'",
                hex(fields.sha1),
                hex(fields.sha256.unwrap())
            );
            let xml = format!("<manifest xmlns='http://www.garmin.com/xml/connectiq' version='3'><application id='our-ephemeral-fixture'><products><product filename='a/app.prg' partNumber='a' {product}/><product filename='b/app.prg' partNumber='b' {product}/></products><unknown retained='yes'/></application></manifest>").into_bytes();
            let signature = key.sign_detached(HashAlgorithm::Sha256, &xml).unwrap();
            Self {
                entries: vec![
                    ("manifest.xml".into(), xml),
                    ("manifest.sig2".into(), signature),
                    (
                        "dev_key.pub".into(),
                        key.public_key().to_spki_der().unwrap(),
                    ),
                    ("a/app.prg".into(), prg.to_bytes()),
                    ("b/app.prg".into(), prg.to_bytes()),
                    ("a/debug.xml".into(), b"<opaque unsigned='true'/>".to_vec()),
                ],
            }
        }
        fn member(&mut self, name: &str) -> &mut Vec<u8> {
            &mut self.entries.iter_mut().find(|(n, _)| n == name).unwrap().1
        }
        fn resign_manifest(&mut self) {
            let signature = keys()[0]
                .sign_detached(HashAlgorithm::Sha256, self.member("manifest.xml"))
                .unwrap();
            *self.member("manifest.sig2") = signature;
        }
        fn bytes(&self) -> Vec<u8> {
            seven_z(
                &self
                    .entries
                    .iter()
                    .map(|(n, b)| (n.as_str(), b.as_slice()))
                    .collect::<Vec<_>>(),
            )
        }
        fn report(&self, expected: Option<&PublicKey>) -> ciq_package::VerificationReport {
            open(self.bytes()).unwrap().verify(expected)
        }
    }
    #[test]
    fn modern_iq_checks_multiple_products_and_optional_identity() {
        let fixture = Fixture::new();
        let report = fixture.report(None);
        assert!(report.verified(), "{report:?}");
        assert!(report.expected_key.is_none());
        assert_eq!(report.products.len(), 2);
        assert_eq!(report.crc_only_members, ["a/debug.xml"]);
        assert!(fixture.report(Some(&keys()[0].public_key())).verified());
        let wrong = fixture.report(Some(&keys()[1].public_key()));
        assert!(!wrong.verified());
        assert!(matches!(wrong.expected_key, Some(Check::Failed(_))));
    }
    #[test]
    fn exact_manifest_bytes_and_detached_signature_tampering_fail() {
        let mut fixture = Fixture::new();
        fixture.member("manifest.xml").push(b'\n');
        let report = fixture.report(None);
        assert!(!report.verified());
        assert!(matches!(report.manifest_signature, Check::Failed(_)));
        assert!(report.archive_integrity.passed());
        fixture.resign_manifest();
        assert!(fixture.report(None).verified());
        fixture.member("manifest.sig2")[0] ^= 1;
        assert!(!fixture.report(None).manifest_signature.passed());
    }
    #[test]
    fn product_code_and_signature_attributes_are_checked_separately() {
        let mut fixture = Fixture::new();
        let mut prg = Prg::parse(fixture.member("a/app.prg")).unwrap();
        prg.replace_same_length(CODE, &[0x38, 42, 0x16]).unwrap();
        *fixture.member("a/app.prg") = prg.to_bytes();
        let report = fixture.report(None);
        assert!(!report.verified());
        assert!(report.manifest_signature.passed());
        assert!(matches!(
            report.products[0].prg_signatures,
            Check::Failed(_)
        ));
        assert!(report.products[1].verified());
        let mut fixture = Fixture::new();
        let xml = String::from_utf8(fixture.member("manifest.xml").clone()).unwrap();
        let at = xml.find(" sig='").unwrap() + 6;
        let mut xml = xml.into_bytes();
        xml[at] = if xml[at] == b'0' { b'1' } else { b'0' };
        *fixture.member("manifest.xml") = xml;
        fixture.resign_manifest();
        let report = fixture.report(None);
        assert!(!report.verified());
        assert!(report.manifest_signature.passed());
        assert!(report.products[0].prg_signatures.passed());
        assert!(matches!(
            report.products[0].product_signatures,
            Check::Failed(_)
        ));
    }
    #[test]
    fn substituted_package_key_and_independently_signed_product_fail_consistency() {
        let mut fixture = Fixture::new();
        *fixture.member("dev_key.pub") = keys()[1].public_key().to_spki_der().unwrap();
        let report = fixture.report(None);
        assert!(!report.verified());
        assert!(!report.manifest_signature.passed());
        assert!(report.products.iter().all(|p| !p.key_consistency.passed()));
        let mut fixture = Fixture::new();
        let prg = ciq_sign::resign(
            &Prg::parse(fixture.member("a/app.prg")).unwrap(),
            &keys()[1],
        )
        .unwrap();
        let old = Prg::parse(fixture.member("a/app.prg")).unwrap();
        let old_fields = ciq_sign::developer_signatures(&old).unwrap();
        let new_fields = ciq_sign::developer_signatures(&prg).unwrap();
        let xml = String::from_utf8(fixture.member("manifest.xml").clone())
            .unwrap()
            .replacen(&hex(old_fields.sha1), &hex(new_fields.sha1), 1)
            .replacen(
                &hex(old_fields.sha256.unwrap()),
                &hex(new_fields.sha256.unwrap()),
                1,
            );
        *fixture.member("manifest.xml") = xml.into_bytes();
        *fixture.member("a/app.prg") = prg.to_bytes();
        fixture.resign_manifest();
        let report = fixture.report(None);
        assert!(report.manifest_signature.passed());
        assert!(report.products[0].prg_signatures.passed());
        assert!(report.products[0].product_signatures.passed());
        assert!(!report.products[0].key_consistency.passed());
        assert!(!report.verified());
    }
    #[test]
    fn missing_malformed_and_unsupported_signing_profiles_never_pass() {
        for member in ["manifest.sig2", "dev_key.pub"] {
            let mut fixture = Fixture::new();
            fixture.entries.retain(|(name, _)| name != member);
            assert!(!fixture.report(None).verified());
        }
        let mut fixture = Fixture::new();
        fixture.entries.retain(|(name, _)| name != "a/app.prg");
        assert!(open(fixture.bytes()).is_err());
        for value in [vec![0; 511], vec![0; 513]] {
            let mut fixture = Fixture::new();
            *fixture.member("manifest.sig2") = value;
            assert!(!fixture.report(None).verified());
        }
        let mut fixture = Fixture::new();
        *fixture.member("dev_key.pub") = vec![1, 2, 3];
        assert!(!fixture.report(None).verified());
        let mut fixture = Fixture::new();
        fixture.entries.push(("manifest.sig".into(), vec![0; 512]));
        assert!(matches!(
            fixture.report(None).manifest_signature,
            Check::Unsupported(_)
        ));
        let mut fixture = Fixture::new();
        let xml = String::from_utf8(fixture.member("manifest.xml").clone())
            .unwrap()
            .replace("rsa-4096", "unknown");
        *fixture.member("manifest.xml") = xml.into_bytes();
        fixture.resign_manifest();
        let report = fixture.report(None);
        assert!(report.manifest_signature.passed());
        assert!(matches!(report.products[0].profile, Check::Unsupported(_)));
        assert!(!report.verified());
        let mut fixture = Fixture::new();
        let mut prg = Prg::parse(fixture.member("a/app.prg"))
            .unwrap()
            .sections()
            .to_vec();
        prg.push(Section::new(ciq_sign::STORE_SIGNATURE, vec![0; 512]).unwrap());
        *fixture.member("a/app.prg") = Prg::from_sections(prg).to_bytes();
        assert!(matches!(
            fixture.report(None).products[0].prg_signatures,
            Check::Unsupported(_)
        ));
    }
    #[test]
    fn invalid_hex_and_legacy_prg_profiles_are_explicit_failures() {
        let mut fixture = Fixture::new();
        let xml = String::from_utf8(fixture.member("manifest.xml").clone()).unwrap();
        let at = xml.find(" sig='").unwrap() + 6;
        let mut xml = xml.into_bytes();
        xml[at] = b'z';
        *fixture.member("manifest.xml") = xml;
        fixture.resign_manifest();
        let report = fixture.report(None);
        assert!(report.manifest_signature.passed());
        assert!(matches!(
            report.products[0].product_signatures,
            Check::Failed(_)
        ));
        assert!(!report.verified());
        let mut fixture = Fixture::new();
        let original = Prg::parse(fixture.member("a/app.prg")).unwrap();
        let mut sections = original.sections().to_vec();
        let last = sections.pop().unwrap();
        sections.push(
            Section::new(
                ciq_sign::DEVELOPER_SIGNATURE,
                last.payload()[..1028].to_vec(),
            )
            .unwrap(),
        );
        *fixture.member("a/app.prg") = Prg::from_sections(sections).to_bytes();
        let report = fixture.report(None);
        assert!(report.products[0].prg_signatures.passed());
        assert!(matches!(report.products[0].profile, Check::Unsupported(_)));
        assert!(!report.verified());
    }
    #[test]
    fn unsigned_debug_edits_pass_signatures_but_archive_damage_does_not() {
        let mut fixture = Fixture::new();
        *fixture.member("a/debug.xml") = b"unsigned edit".to_vec();
        assert!(fixture.report(None).verified());
        let mut data = fixture.bytes();
        let at = data
            .windows(13)
            .position(|b| b == b"unsigned edit")
            .unwrap();
        data[at] ^= 1;
        let report = open(data).unwrap().verify(None);
        assert!(matches!(report.archive_integrity, Check::Failed(_)));
        assert!(!report.verified());
        let mut barrel = open(zip(&barrel_entries())).unwrap();
        let report = barrel.verify(None);
        assert!(report.archive_integrity.passed());
        assert!(matches!(report.manifest_signature, Check::Unsupported(_)));
        assert!(!report.verified());
    }
    #[test]
    fn verify_cli_reports_checks_and_fails_for_wrong_expected_key() {
        let fixture = Fixture::new();
        let temp = Temp::new();
        let archive = temp.0.join("application.iq");
        fs::write(&archive, fixture.bytes()).unwrap();
        let expected = temp.0.join("expected.pub");
        fs::write(&expected, keys()[0].public_key().to_spki_der().unwrap()).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_ciq-package"))
            .arg("verify")
            .arg(&archive)
            .arg("--expected-key")
            .arg(&expected)
            .output()
            .unwrap();
        assert!(result.status.success(), "{:?}", result);
        let output = String::from_utf8(result.stdout).unwrap();
        assert!(output.contains("Manifest SHA-256 signature: passed"));
        assert!(output.contains("Expected public key: passed"));
        assert!(output.contains("a/debug.xml"));
        fs::write(&expected, keys()[1].public_key().to_spki_der().unwrap()).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_ciq-package"))
            .arg("verify")
            .arg(&archive)
            .arg("--expected-key")
            .arg(expected)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 2);
    }
}

fn packing_tree(temp: &Temp, reverse: bool) -> PathBuf {
    let root = temp.0.join("source");
    fs::create_dir(&root).unwrap();
    let mut entries = barrel_entries();
    if reverse {
        entries.reverse();
    }
    for (name, bytes) in entries {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fs::create_dir(root.join("empty")).unwrap();
    fs::write(root.join("zero.bin"), []).unwrap();
    root
}
#[test]
fn packed_barrel_is_deterministic_and_preserves_payloads() {
    let a = Temp::new();
    let b = Temp::new();
    let source_a = packing_tree(&a, false);
    let source_b = packing_tree(&b, true);
    let file = fs::File::open(source_b.join("unknown.bin")).unwrap();
    file.set_times(fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH))
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            source_b.join("unknown.bin"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
    }
    ciq_package::pack_barrel(&source_a, a.output(), Limits::default()).unwrap();
    ciq_package::pack_barrel(&source_b, b.output(), Limits::default()).unwrap();
    assert_eq!(fs::read(a.output()).unwrap(), fs::read(b.output()).unwrap());
    let cli_output = a.0.join("cli.barrel");
    let result = Command::new(env!("CARGO_BIN_EXE_ciq-package"))
        .arg("pack-barrel")
        .arg(&source_a)
        .arg(&cli_output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(fs::read(a.output()).unwrap(), fs::read(cli_output).unwrap());
    let mut package = Package::open(a.output(), Limits::default()).unwrap();
    let extracted = a.0.join("extracted");
    package.unpack(&extracted).unwrap();
    for (name, bytes) in barrel_entries() {
        assert_eq!(fs::read(extracted.join(name)).unwrap(), bytes);
    }
    assert!(extracted.join("empty").is_dir());
    assert_eq!(fs::read(extracted.join("zero.bin")).unwrap(), []);
    let names: Vec<_> = package.members().iter().map(|m| m.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
}
#[test]
fn packing_rejects_mismatched_or_invalid_manifests_before_output() {
    let temp = Temp::new();
    let source = packing_tree(&temp, false);
    fs::write(
        source.join("content/manifest.xml"),
        [BARREL, b"\n"].concat(),
    )
    .unwrap();
    assert!(
        ciq_package::pack_barrel(&source, temp.output(), Limits::default())
            .unwrap_err()
            .to_string()
            .contains("identical")
    );
    assert!(!temp.output().exists());
    for name in ["manifest.xml", "content/manifest.xml"] {
        fs::write(source.join(name), IQ).unwrap();
    }
    assert!(ciq_package::pack_barrel(&source, temp.output(), Limits::default()).is_err());
    assert!(!temp.output().exists());
}
#[test]
fn packing_refuses_existing_output_and_output_inside_source() {
    let temp = Temp::new();
    let source = packing_tree(&temp, false);
    fs::write(temp.output(), b"keep me").unwrap();
    assert!(ciq_package::pack_barrel(&source, temp.output(), Limits::default()).is_err());
    assert_eq!(fs::read(temp.output()).unwrap(), b"keep me");
    assert!(
        ciq_package::pack_barrel(&source, source.join("new.barrel"), Limits::default()).is_err()
    );
    assert!(!source.join("new.barrel").exists());
}
#[test]
fn packing_enforces_input_and_output_limits() {
    let temp = Temp::new();
    let source = packing_tree(&temp, false);
    for limits in [
        Limits {
            entries: 2,
            ..Limits::default()
        },
        Limits {
            member_bytes: 10,
            ..Limits::default()
        },
        Limits {
            total_bytes: 10,
            ..Limits::default()
        },
        Limits {
            manifest_bytes: 10,
            ..Limits::default()
        },
        Limits {
            archive_bytes: 64,
            ..Limits::default()
        },
        Limits {
            header_bytes: 1,
            ..Limits::default()
        },
    ] {
        assert!(ciq_package::pack_barrel(&source, temp.output(), limits).is_err());
        assert!(!temp.output().exists());
    }
}
#[cfg(unix)]
#[test]
fn packing_rejects_symlinks_and_case_collisions() {
    let temp = Temp::new();
    let source = packing_tree(&temp, false);
    let link = temp.0.join("link");
    std::os::unix::fs::symlink(&source, &link).unwrap();
    assert!(ciq_package::pack_barrel(link, temp.output(), Limits::default()).is_err());
    std::os::unix::fs::symlink(source.join("unknown.bin"), source.join("alias")).unwrap();
    assert!(ciq_package::pack_barrel(&source, temp.output(), Limits::default()).is_err());
    fs::remove_file(source.join("alias")).unwrap();
    fs::write(source.join("UNKNOWN.bin"), b"collision").unwrap();
    assert!(ciq_package::pack_barrel(&source, temp.output(), Limits::default()).is_err());
    assert!(!temp.output().exists());
}

#[test]
fn encoded_lzma_headers_are_checked_before_backend_allocation() {
    let mut writer = sevenz_rust2::ArchiveWriter::new(Cursor::new(Vec::new())).unwrap();
    writer.set_encrypt_header(false);
    for (name, bytes) in iq_entries() {
        writer
            .push_archive_entry(
                sevenz_rust2::ArchiveEntry {
                    name: name.into(),
                    ..Default::default()
                },
                Some(bytes),
            )
            .unwrap();
    }
    let data = writer.finish().unwrap().into_inner();
    let at = 32 + u64::from_le_bytes(data[12..20].try_into().unwrap()) as usize;
    assert_eq!(data[at], 23);
    assert!(open(data.clone()).is_ok());
    let encoded_size = u64::from_le_bytes(data[20..28].try_into().unwrap()) as usize;
    assert!(
        Package::read(
            data.as_slice(),
            Limits {
                header_bytes: encoded_size,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(
        Package::read(
            data.as_slice(),
            Limits {
                dictionary_bytes: 4096,
                ..Limits::default()
            }
        )
        .is_err()
    );
    let mut corrupt = data.clone();
    corrupt[at - 1] ^= 1;
    assert!(open(corrupt).is_err());
    // Valid outer CRCs do not permit an arbitrary encoded-header graph.
    assert!(open(envelope(&data[32..at], &[23, 6, 0, 2])).is_err());
}
