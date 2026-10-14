use ciq_prg::{
    CODE, DATA, Prg, SYMBOLS, Section,
    manifest::{Encoding, FILE_NAME, Manifest},
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn null_checks_export_as_assembly_and_reconstruct_with_stable_disassembly() {
    let temporary = TemporaryDirectory::new();
    let first = temporary.0.join("first");
    let second = temporary.0.join("second");
    let original = Prg::from_sections(vec![
        Section::new(CODE, vec![0x12, 1, 0x20, 0x16, 0x12, 1, 0x4b, 0x16]).unwrap(),
    ]);
    let manifest = ciq_dis::export_project(&original, &first).unwrap();
    assert_eq!(manifest.entries[0].encoding, Encoding::Assembly);
    let text = fs::read_to_string(first.join(&manifest.entries[0].file)).unwrap();
    assert!(text.contains("isnull"));
    assert!(text.contains("isnotnull"));
    let rebuilt = ciq_asm::assemble_project(&first).unwrap();
    assert_eq!(rebuilt, original);
    let repeated = ciq_dis::export_project(&rebuilt, &second).unwrap();
    assert_eq!(repeated.to_text(), manifest.to_text());
    assert_eq!(
        fs::read_to_string(second.join(&repeated.entries[0].file)).unwrap(),
        text
    );
}

#[test]
fn exception_labels_include_code_end_and_resolve_independently_of_section_order() {
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("exceptions");
    let payload = ciq_prg::exceptions::Source::parse(".exception begin=0x00 end=0x02 handler=0x01\n.exception begin=0xffffff end=0x00 handler=0x02").unwrap().encode(&Default::default()).unwrap();
    let original = Prg::from_sections(vec![
        Section::new(ciq_prg::EXCEPTIONS, payload).unwrap(),
        Section::new(CODE, vec![0x16, 0x16]).unwrap(),
    ]);
    let manifest = ciq_dis::export_project(&original, &directory).unwrap();
    assert_eq!(manifest.entries[0].encoding, Encoding::Exceptions);
    assert!(manifest.to_text().starts_with("ciq-project"));
    let path = directory.join(&manifest.entries[0].file);
    let source = fs::read_to_string(&path).unwrap();
    assert!(
        source
            .contains("begin=code:code_00000000 end=code:code_00000002 handler=code:code_00000001")
    );
    assert!(source.contains("begin=0xffffff end=code:code_00000000 handler=0x000002"));
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
    let code_path = directory.join(&manifest.entries[1].file);
    let code = fs::read_to_string(&code_path).unwrap();
    fs::write(
        &code_path,
        code.replace("code_00000001:", "popv\ncode_00000001:"),
    )
    .unwrap();
    let mut larger = manifest.clone();
    larger.entries[1].length = 3;
    fs::write(directory.join(FILE_NAME), larger.to_text()).unwrap();
    let moved = ciq_asm::assemble_project(&directory).unwrap();
    let ex = ciq_prg::exceptions::Source::from_payload(
        moved.unique_section(ciq_prg::EXCEPTIONS).unwrap().payload(),
    )
    .unwrap();
    assert_eq!(ex.entries[0].handler, ciq_prg::exceptions::Offset::Raw(2));
    assert_eq!(ex.entries[0].end, ciq_prg::exceptions::Offset::Raw(3));
    for (old, new) in [
        ("begin=code:code_00000000", "begin=code:code_00000002"),
        ("handler=code:code_00000001", "handler=code:code_00000002"),
        ("handler=code:code_00000001", "handler=code:missing"),
    ] {
        fs::write(&path, source.replace(old, new)).unwrap();
        assert!(ciq_asm::assemble_project(&directory).is_err());
    }
    let mut duplicate = larger;
    duplicate.entries.push(duplicate.entries[0].clone());
    fs::write(directory.join(FILE_NAME), duplicate.to_text()).unwrap();
    assert!(
        ciq_asm::assemble_project(&directory)
            .unwrap_err()
            .to_string()
            .contains("structured EXCEPTIONS requires a unique section")
    );
}

#[test]
fn malformed_duplicate_and_unsupported_code_exceptions_preserve_raw_inputs() {
    for payloads in [vec![vec![0, 0, 1]], vec![vec![0, 0], vec![0, 0]]] {
        let temporary = TemporaryDirectory::new();
        let directory = temporary.0.join("raw");
        let original = Prg::from_sections(
            payloads
                .into_iter()
                .map(|p| Section::new(ciq_prg::EXCEPTIONS, p).unwrap())
                .collect(),
        );
        let manifest = ciq_dis::export_project(&original, &directory).unwrap();
        assert!(manifest.entries.iter().all(|e| e.encoding == Encoding::Raw));
        assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
    }
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("unsupported-code");
    let original = Prg::from_sections(vec![
        Section::new(CODE, vec![0xff]).unwrap(),
        Section::new(ciq_prg::EXCEPTIONS, vec![0, 0]).unwrap(),
    ]);
    ciq_dis::export_project(&original, &directory).unwrap();
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
}

#[test]
fn branch_targets_export_as_labels_and_invalid_targets_retain_numeric_bits() {
    // Forward to return, backward to start, inside an operand, negative, and beyond CODE.
    let code = vec![
        0x29, 0, 12, 0x19, 0xff, 0xfa, 0x19, 0xff, 0xff, 0x19, 0xff, 0xf3, 0x23, 0x7f, 0xff, 0x16,
    ];
    let original = Prg::from_sections(vec![Section::new(CODE, code.clone()).unwrap()]);
    let symbolic = ciq_dis::disassemble_symbolic(&original).unwrap();
    assert!(symbolic.contains("bf code:code_0000000f"));
    assert!(symbolic.contains("goto code:code_00000000"));
    for bits in ["goto 0xffff", "goto 0xfff3", "jsr 0x7fff"] {
        assert!(symbolic.contains(bits));
    }
    assert_eq!(ciq_asm::assemble(&symbolic).unwrap(), code);
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("project");
    ciq_dis::export_project(&original, &directory).unwrap();
    let exported = fs::read_to_string(directory.join("000-code-c0debabe.mcasm")).unwrap();
    assert!(exported.contains("bf code:code_0000000f"));
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
    let end_target = Prg::from_sections(vec![Section::new(CODE, vec![0x19, 0, 0]).unwrap()]);
    let listing = ciq_dis::disassemble_symbolic(&end_target).unwrap();
    assert!(listing.contains("goto 0x0000"));
    assert!(!listing.contains("code:"));
    assert_eq!(ciq_asm::assemble(&listing).unwrap(), [0x19, 0, 0]);
}

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
            .starts_with("ciq-project\n")
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

fn string_specimen() -> Prg {
    // The prefix is opaque, so extraction must use the news reference.
    Prg::from_sections(vec![
        Section::new(DATA, vec![0xff, 0xc1, 1, 0, 2, b'h', b'i', 0, 0xee]).unwrap(),
        Section::new(CODE, vec![0x18, 0, 0, 0, 2, 0x16]).unwrap(),
        Section::new(ciq_prg::RESOURCES, vec![0, 0]).unwrap(),
        Section::new(0xe1c0de12, vec![0xaa, 0xbb]).unwrap(),
    ])
}

#[test]
fn data_source_is_authoritative_preserves_opaque_bytes_and_rejects_relocation() {
    let original = string_specimen();
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("project");
    let manifest = ciq_dis::export_project(&original, &directory).unwrap();
    assert_eq!(manifest.entries[0].encoding, Encoding::Data);
    let code = fs::read_to_string(directory.join(&manifest.entries[1].file)).unwrap();
    assert!(code.contains("news &str_00000002"));
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
    let path = directory.join(&manifest.entries[0].file);
    let text = fs::read_to_string(&path).unwrap();
    fs::write(&path, text.replace("\"hi\"", "\"yo\"")).unwrap();
    let edited = ciq_asm::assemble_project(&directory).unwrap();
    assert_eq!(
        edited.sections()[0].payload(),
        [0xff, 0xc1, 1, 0, 2, b'y', b'o', 0, 0xee]
    );
    assert_eq!(edited.sections()[1..], original.sections()[1..]);
    fs::write(&path, text.replace("\"hi\"", "\"longer\"")).unwrap();
    assert!(ciq_asm::assemble_project(&directory).is_err());
    fs::write(&path, text).unwrap();
    fs::write(
        directory.join(&manifest.entries[1].file),
        format!(".string inline \"hi\" at=0x2\n{code}"),
    )
    .unwrap();
    assert!(
        ciq_asm::assemble_project(&directory)
            .unwrap_err()
            .to_string()
            .contains("authoritative DATA")
    );
}

#[test]
fn symbolic_template_applies_inline_string_edits_with_typed_references() {
    let original = string_specimen();
    let text = ciq_dis::disassemble_symbolic(&original).unwrap();
    assert!(text.contains(".string str_00000002 \"hi\" at=0x00000002"));
    assert_eq!(
        ciq_asm::assemble_template(original.clone(), &text).unwrap(),
        original
    );
    let edited =
        ciq_asm::assemble_template(original.clone(), &text.replace("\"hi\"", "\"yo\"")).unwrap();
    assert_eq!(
        ciq_prg::data::string_at(edited.sections()[0].payload(), 2)
            .unwrap()
            .0,
        "yo"
    );
    assert_eq!(edited.sections()[1..], original.sections()[1..]);
    assert!(ciq_asm::assemble_template(original, &text.replace("\"hi\"", "\"longer\"")).is_err());
    for instruction in ["getselfv &x", "argc &x", "news @x", "news &missing"] {
        assert!(ciq_asm::assemble(&format!(".string x \"hi\" at=0x2\n{instruction}")).is_err());
    }
    assert!(ciq_asm::assemble(".string x \"hi\"\nnews &x").is_err());
}

#[test]
fn invalid_data_references_and_duplicate_sections_preserve_raw_input() {
    for payload in [vec![1, 0, 1, b'x'], vec![2, 0, 0, 0]] {
        let original = Prg::from_sections(vec![
            Section::new(DATA, payload).unwrap(),
            Section::new(CODE, vec![0x18, 0, 0, 0, 0]).unwrap(),
        ]);
        let temporary = TemporaryDirectory::new();
        let directory = temporary.0.join("invalid");
        let manifest = ciq_dis::export_project(&original, &directory).unwrap();
        assert_eq!(manifest.entries[0].encoding, Encoding::Raw);
        assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
    }
    let duplicate = Prg::from_sections(vec![
        Section::new(DATA, vec![1, 0, 0, 0]).unwrap(),
        Section::new(DATA, vec![1, 0, 0, 0]).unwrap(),
    ]);
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("duplicate");
    let manifest = ciq_dis::export_project(&duplicate, &directory).unwrap();
    assert!(
        manifest
            .entries
            .iter()
            .all(|entry| entry.encoding == Encoding::Raw)
    );
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), duplicate);
}

fn class_specimen() -> Prg {
    let mut data = vec![0xc2, 0xa5, 0x5d, 0xef, 0, 0, 0, 0, 4];
    for (kind, value) in [
        (6, 0x10000002u32),
        (6, 0x10000001),
        (3, 45),
        (6, 0x40010024),
    ] {
        data.extend_from_slice(&0x80006710u32.to_be_bytes());
        data.push(kind);
        data.extend_from_slice(&value.to_be_bytes());
    }
    data.extend_from_slice(&[1, 0, 3, b'x', b'\n', b'"', 0]);
    Prg::from_sections(vec![
        Section::new(DATA, data).unwrap(),
        Section::new(CODE, vec![0x38, 1, 0x16]).unwrap(),
        Section::new(SYMBOLS, symbol_payload()).unwrap(),
    ])
}

#[test]
fn class_listing_resolves_typed_references_and_checks_instruction_boundaries() {
    let original = class_specimen();
    let listing = ciq_dis::classes(&original).unwrap();
    assert!(listing.contains("symbol=0x00800067 \"print\\n\\\"ln\""));
    assert!(listing.contains("value=0x10000002 -> CODE+0x00000002\n"));
    assert!(listing.contains("CODE+0x00000001 [not an instruction boundary]"));
    assert!(listing.contains("DATA+0x0000002d \"x\\n\\\"\""));
    assert!(listing.contains("value=0x40010024\n"));
    let without_symbols = Prg::from_sections(original.sections()[..2].to_vec());
    assert!(
        ciq_dis::classes(&without_symbols)
            .unwrap()
            .contains("symbol=0x00800067 key_bits=")
    );
    let without_code = Prg::from_sections(vec![original.sections()[0].clone()]);
    assert!(
        ciq_dis::classes(&without_code)
            .unwrap()
            .contains("[boundary unchecked]")
    );
}

#[test]
fn class_sidecar_is_inspection_only_and_never_changes_project_import() {
    let original = class_specimen();
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("project");
    let manifest = ciq_dis::export_project(&original, &directory).unwrap();
    let sidecar = directory.join("000-data.classes.txt");
    assert!(fs::read_to_string(&sidecar).unwrap().contains("class-v2"));
    assert!(
        manifest
            .entries
            .iter()
            .all(|entry| !entry.file.ends_with(".classes.txt"))
    );
    fs::write(sidecar, "edited inspection text, not assembly").unwrap();
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
}

#[test]
fn shared_method_labels_roundtrip_and_reject_missing_or_end_targets() {
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("labels");
    let data = ciq_prg::data::Source::parse(".class app version=0x02 extends=none statics=none parent=none module=none app_flags=0x0\n.field key=0x8146 type=0x06 value=0x10000000\n.endclass\n").unwrap().encode().unwrap().0;
    let original = Prg::from_sections(vec![
        Section::new(DATA, data).unwrap(),
        Section::new(CODE, vec![0x35, 1, 0x38, 42, 0x16]).unwrap(),
    ]);
    let manifest = ciq_dis::export_project(&original, &directory).unwrap();
    let data_path = directory.join(&manifest.entries[0].file);
    let code_path = directory.join(&manifest.entries[1].file);
    let text = fs::read_to_string(&data_path).unwrap();
    assert!(text.contains("value=code:code_00000000"));
    assert!(
        fs::read_to_string(&code_path)
            .unwrap()
            .starts_with("code_00000000:\n")
    );
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
    fs::write(
        &data_path,
        text.replace("code:code_00000000", "code:missing"),
    )
    .unwrap();
    assert!(ciq_asm::assemble_project(&directory).is_err());
    fs::write(&data_path, text.replace("code:code_00000000", "code:end")).unwrap();
    fs::write(
        &code_path,
        "code_00000000:\nargc 0x01\nipush1 0x2a\nreturn\nend:\n",
    )
    .unwrap();
    assert!(ciq_asm::assemble_project(&directory).is_err());
}

#[test]
fn structured_debug_and_line_labels_roundtrip_and_move_together() {
    use ciq_prg::debug::{DebugSource, LineSource, Pc, StringRef};
    let debug = DebugSource::parse(".string file \"file.mc\"\n.string function \"probe\"\n")
        .unwrap()
        .encode()
        .unwrap()
        .0;
    let lines = LineSource::parse(".line pc=0x10000000 file=0x0 function=0xb line=1\n.line pc=0x10000002 file=0x0 function=0xb line=2\n").unwrap().encode(&Default::default(),&Default::default()).unwrap();
    let original = Prg::from_sections(vec![
        Section::new(0xc0de7ab1, lines).unwrap(),
        Section::new(0xd0000d1e, debug).unwrap(),
        Section::new(CODE, vec![0x38, 42, 0x16]).unwrap(),
    ]);
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("metadata");
    let mut manifest = ciq_dis::export_project(&original, &directory).unwrap();
    assert_eq!(manifest.entries[0].encoding, Encoding::Lines);
    assert_eq!(manifest.entries[1].encoding, Encoding::Debug);
    assert!(manifest.to_text().starts_with("ciq-project"));
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
    let code_path = directory.join(&manifest.entries[2].file);
    let text = fs::read_to_string(&code_path).unwrap();
    fs::write(&code_path, format!("ipushz\npopv\n{text}")).unwrap();
    manifest.entries[2].length += 2;
    fs::write(directory.join(FILE_NAME), manifest.to_text()).unwrap();
    let changed = ciq_asm::assemble_project(&directory).unwrap();
    let lines =
        LineSource::from_payload(changed.unique_section(0xc0de7ab1).unwrap().payload()).unwrap();
    assert_eq!(lines.entries[0].pc, Pc::Raw(0x10000002));
    assert_eq!(lines.entries[1].pc, Pc::Raw(0x10000004));
    assert_eq!(
        changed.unique_section(0xd0000d1e).unwrap(),
        original.unique_section(0xd0000d1e).unwrap()
    );
    let line_path = directory.join(&manifest.entries[0].file);
    let text = fs::read_to_string(&line_path).unwrap();
    let debug_path = directory.join(&manifest.entries[1].file);
    fs::write(
        &debug_path,
        ".string dbg_00000000 \"x\"\n.string dbg_0000000b \"probe\"\n",
    )
    .unwrap();
    manifest.entries[1].length -= 6;
    fs::write(directory.join(FILE_NAME), manifest.to_text()).unwrap();
    let changed = ciq_asm::assemble_project(&directory).unwrap();
    let lines =
        LineSource::from_payload(changed.unique_section(0xc0de7ab1).unwrap().payload()).unwrap();
    assert_eq!(lines.entries[0].function, StringRef::Raw(5));
    assert_eq!(lines.entries[1].function, StringRef::Raw(5));
    fs::write(
        &code_path,
        format!("{}end:\n", fs::read_to_string(&code_path).unwrap()),
    )
    .unwrap();
    fs::write(&line_path, text.replace("code:code_00000000", "code:end")).unwrap();
    assert!(
        ciq_asm::assemble_project(&directory)
            .unwrap_err()
            .to_string()
            .contains("not an instruction")
    );
    fs::write(
        &line_path,
        text.replace("code:code_00000000", "code:missing"),
    )
    .unwrap();
    assert!(ciq_asm::assemble_project(&directory).is_err());
}

#[test]
fn malformed_debug_and_line_sections_fall_back_to_raw() {
    let original = Prg::from_sections(vec![
        Section::new(0xd0000d1e, vec![3, 0]).unwrap(),
        Section::new(0xc0de7ab1, vec![0, 1]).unwrap(),
        Section::new(CODE, vec![0x16]).unwrap(),
    ]);
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("raw_metadata");
    let manifest = ciq_dis::export_project(&original, &directory).unwrap();
    assert_eq!(manifest.entries[0].encoding, Encoding::Raw);
    assert_eq!(manifest.entries[1].encoding, Encoding::Raw);
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
}

#[test]
fn owner_qualified_labels_roundtrip_a_shared_member_symbol() {
    let symbols = ciq_prg::symbols::source::Source::parse(
        ".symbol Left \"Left\" id=0x01\n.symbol Right \"Right\" id=0x02\n.symbol answer \"answer\" id=0x03\n",
    ).unwrap();
    let symbol_payload = symbols.encode().unwrap();
    let bindings = symbols.bindings().unwrap();
    let data = ciq_prg::data::Source::parse_with_symbols(
        ".class Left version=0x02 extends=none statics=none parent=none module=@Left app_flags=0x0\n.field symbol=@answer bits=0x10 type=0x06 value=0x10000000\n.endclass\n.class Right version=0x02 extends=none statics=none parent=none module=@Right app_flags=0x0\n.field symbol=@answer bits=0x10 type=0x06 value=0x10000003\n.endclass\n", &bindings,
    ).unwrap().encode_with_symbols(&bindings).unwrap().0;
    let original = Prg::from_sections(vec![
        Section::new(SYMBOLS, symbol_payload).unwrap(),
        Section::new(DATA, data).unwrap(),
        Section::new(CODE, vec![0x38, 21, 0x16, 0x38, 22, 0x16]).unwrap(),
    ]);
    let temporary = TemporaryDirectory::new();
    let directory = temporary.0.join("owners");
    let manifest = ciq_dis::export_project(&original, &directory).unwrap();
    let data = fs::read_to_string(directory.join(&manifest.entries[1].file)).unwrap();
    let code = fs::read_to_string(directory.join(&manifest.entries[2].file)).unwrap();
    for label in ["method_Left__answer", "method_Right__answer"] {
        assert!(data.contains(&format!("value=code:{label}")));
        assert!(code.contains(&format!("{label}:")));
    }
    assert_eq!(ciq_asm::assemble_project(&directory).unwrap(), original);
}
