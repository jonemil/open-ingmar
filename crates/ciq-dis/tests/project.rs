use ciq_prg::{
    CODE, Prg, Section,
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
