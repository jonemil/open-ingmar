use ciq_prg::{
    CODE, Prg, SYMBOLS, Section,
    manifest::{Encoding, FILE_NAME, Manifest},
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ciq-project-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

fn symbol_payload() -> Vec<u8> {
    let mut bytes = vec![0, 2];
    bytes.extend_from_slice(&0x800005u32.to_be_bytes());
    bytes.extend_from_slice(&18u32.to_be_bytes());
    bytes.extend_from_slice(&0x800067u32.to_be_bytes());
    bytes.extend_from_slice(&35u32.to_be_bytes());
    for name in ["Toybox_System", "print\n\"ln"] {
        bytes.push(1);
        bytes.extend_from_slice(&(name.len() as u16).to_be_bytes());
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(0);
    }
    bytes
}

#[test]
fn symbol_annotations_are_escaped_and_do_not_change_assembled_bytes() {
    let code = vec![
        0x45, 0, 0x80, 0, 5, 0, 0x80, 0, 0x67, 0x18, 0, 0x80, 0, 5, 0x38, 5, 0x16,
    ];
    let original = Prg::from_sections(vec![
        Section::new(CODE, code.clone()).unwrap(),
        Section::new(SYMBOLS, symbol_payload()).unwrap(),
    ]);
    let listing = ciq_dis::disassemble_prg(&original).unwrap();
    assert!(listing.contains("module=\"Toybox_System\"; member=\"print\\n\\\"ln\""));
    assert_eq!(listing.lines().count(), 4);
    // A pointer and immediate with matching numeric values aren't symbol operands.
    assert!(!listing.lines().nth(1).unwrap().contains("symbol="));
    assert!(!listing.lines().nth(2).unwrap().contains("symbol="));
    assert_eq!(ciq_asm::assemble(&listing).unwrap(), code);
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("project");
    ciq_dis::export_project(&original, &directory).unwrap();
    let symbols = fs::read_to_string(directory.join("001-symbols.symbols.txt")).unwrap();
    assert!(symbols.contains("print\\n\\\"ln"));
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
}

#[test]
fn missing_or_malformed_symbols_do_not_prevent_raw_preservation() {
    let code = vec![0x43, 0, 0, 0, 0x80, 0x16];
    let without = Prg::from_sections(vec![Section::new(CODE, code.clone()).unwrap()]);
    let expected = ciq_dis::disassemble(&code).unwrap();
    assert_eq!(ciq_dis::disassemble_prg(&without).unwrap(), expected);
    let original = Prg::from_sections(vec![
        Section::new(CODE, code).unwrap(),
        Section::new(SYMBOLS, vec![0, 1]).unwrap(),
    ]);
    assert_eq!(ciq_dis::disassemble_prg(&original).unwrap(), expected);
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("project");
    ciq_dis::export_project(&original, &directory).unwrap();
    assert!(!directory.join("001-symbols.symbols.txt").exists());
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn specimen() -> Prg {
    Prg::from_sections(vec![
        Section::new(0xd000d00d, vec![1, 5, 2, 0]).unwrap(),
        Section::new(0x12345678, vec![0xff, 0, 0x35]).unwrap(),
        Section::new(CODE, vec![0x35, 1, 0x38, 41, 0x16]).unwrap(),
        Section::new(0x12345678, vec![]).unwrap(),
        Section::new(0xe1c0de12, vec![0xaa, 0xbb, 0xcc]).unwrap(),
    ])
}

#[test]
fn complete_roundtrip_preserves_order_duplicates_unknown_bytes_and_signatures() {
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("project");
    let original = specimen();
    let manifest = ciq_dis::export_project(&original, &directory).unwrap();
    assert_eq!(manifest.entries[2].encoding, Encoding::Assembly);
    assert_eq!(manifest.entries[1].encoding, Encoding::Raw);
    let parsed = Manifest::parse(&fs::read_to_string(directory.join(FILE_NAME)).unwrap()).unwrap();
    assert_eq!(parsed, manifest);
    let rebuilt = ciq_asm::assemble_project(&directory).unwrap();
    assert_eq!(rebuilt.to_bytes(), original.to_bytes());
    assert!(ciq_dis::export_project(&original, &directory).is_err());
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
}

#[test]
fn unsupported_code_falls_back_to_raw_without_losing_bytes() {
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("project");
    let original = Prg::from_sections(vec![Section::new(CODE, vec![0xff, 0x35, 1]).unwrap()]);
    let manifest = ciq_dis::export_project(&original, &directory).unwrap();
    assert_eq!(manifest.entries[0].encoding, Encoding::Raw);
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
}

#[test]
fn equal_length_code_edit_changes_only_code_and_length_changes_are_rejected() {
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("project");
    let original = specimen();
    let manifest = ciq_dis::export_project(&original, &directory).unwrap();
    let path = directory.join(&manifest.entries[2].file);
    let text = fs::read_to_string(&path).unwrap();
    fs::write(&path, text.replace("0x29", "0x2a")).unwrap();
    let rebuilt = ciq_asm::assemble_project(&directory).unwrap();
    assert_eq!(rebuilt.sections()[2].payload(), [0x35, 1, 0x38, 42, 0x16]);
    for index in [0, 1, 3, 4] {
        assert_eq!(rebuilt.sections()[index], original.sections()[index]);
    }
    fs::write(&path, "return\n").unwrap();
    assert!(
        ciq_asm::assemble_project(&directory)
            .unwrap_err()
            .to_string()
            .contains("manifest requires")
    );
    fs::write(&path, text).unwrap();
    fs::write(directory.join(&manifest.entries[1].file), [1]).unwrap();
    assert!(ciq_asm::assemble_project(&directory).is_err());
}

#[cfg(unix)]
#[test]
fn import_rejects_section_and_manifest_symlinks_outside_project() {
    use std::os::unix::fs::symlink;
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("project");
    let manifest = ciq_dis::export_project(&specimen(), &directory).unwrap();
    let outside = temporary.0.join("outside.bin");
    fs::write(&outside, [1, 2, 3]).unwrap();
    let path = directory.join(&manifest.entries[1].file);
    fs::remove_file(&path).unwrap();
    symlink(&outside, &path).unwrap();
    assert!(
        ciq_asm::assemble_project(&directory)
            .unwrap_err()
            .to_string()
            .contains("outside")
    );
    let path = directory.join(FILE_NAME);
    fs::remove_file(&path).unwrap();
    symlink(outside, path).unwrap();
    assert!(
        ciq_asm::assemble_project(&directory)
            .unwrap_err()
            .to_string()
            .contains("outside")
    );
}

#[test]
fn structured_symbols_are_authoritative_and_named_code_tracks_alias_changes() {
    let original = Prg::from_sections(vec![
        Section::new(CODE, vec![0x45, 0, 0x80, 0, 5, 0, 0x80, 0, 0x67, 0x16]).unwrap(),
        Section::new(SYMBOLS, symbol_payload()).unwrap(),
    ]);
    let standalone = ciq_dis::disassemble_symbolic(&original).unwrap();
    assert!(standalone.contains("getmv @Toybox_System @sym_00800067"));
    assert_eq!(
        ciq_asm::assemble(&standalone).unwrap(),
        original.sections()[0].payload()
    );
    let text = ciq_dis::symbol_source(&original).unwrap();
    assert_eq!(
        ciq_prg::symbols::source::Source::parse(&text)
            .unwrap()
            .encode()
            .unwrap(),
        original.sections()[1].payload()
    );
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("project");
    let manifest = ciq_dis::export_project(&original, &directory).unwrap();
    assert_eq!(manifest.entries[1].encoding, Encoding::Symbols);
    assert!(
        fs::read_to_string(directory.join(FILE_NAME))
            .unwrap()
            .starts_with("ciq-project 2\n")
    );
    let code_path = directory.join(&manifest.entries[0].file);
    let symbol_path = directory.join(&manifest.entries[1].file);
    let code = fs::read_to_string(&code_path).unwrap();
    let symbols = fs::read_to_string(&symbol_path).unwrap();
    fs::write(&code_path, code.replace("@Toybox_System", "@system")).unwrap();
    fs::write(
        &symbol_path,
        symbols.replace(".symbol Toybox_System ", ".symbol system "),
    )
    .unwrap();
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
    // Equal-width content editing is consumed, while unrelated sections are preserved.
    let symbols = fs::read_to_string(&symbol_path).unwrap();
    fs::write(
        &symbol_path,
        symbols.replace("\"Toybox_System\"", "\"Toybox_Systen\""),
    )
    .unwrap();
    let edited = ciq_asm::assemble_project(&directory).unwrap();
    assert_eq!(edited.sections()[0], original.sections()[0]);
    let parsed = ciq_prg::symbols::Symbols::parse(edited.sections()[1].payload()).unwrap();
    assert_eq!(parsed.get(0x800005).unwrap().name, "Toybox_Systen");
    fs::write(&symbol_path, symbols.replace("\"Toybox_System\"", "\"X\"")).unwrap();
    assert!(ciq_asm::assemble_project(&directory).is_err());
}

#[test]
fn unreferenced_names_and_duplicate_symbol_sections_fall_back_to_raw() {
    let mut payload = symbol_payload();
    payload.extend_from_slice(&[1, 0, 1, b'x', 0]);
    let original = Prg::from_sections(vec![
        Section::new(CODE, vec![0x43, 0, 0x80, 0, 5]).unwrap(),
        Section::new(SYMBOLS, payload).unwrap(),
    ]);
    assert!(ciq_dis::symbol_source(&original).is_err());
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("orphan");
    let manifest = ciq_dis::export_project(&original, &directory).unwrap();
    assert_eq!(manifest.entries[1].encoding, Encoding::Raw);
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
    let duplicate = Prg::from_sections(vec![
        Section::new(SYMBOLS, symbol_payload()).unwrap(),
        Section::new(SYMBOLS, symbol_payload()).unwrap(),
    ]);
    let directory = temporary.0.join("duplicate");
    let manifest = ciq_dis::export_project(&duplicate, &directory).unwrap();
    assert!(manifest.entries.iter().all(|e| e.encoding == Encoding::Raw));
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), duplicate);
}
