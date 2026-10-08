use ciq_prg::{CODE, Prg, Section};
use rand_core::OsRng;
use rsa::{RsaPrivateKey, pkcs8::EncodePrivateKey};
use std::{
    ffi::OsStr,
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn run(args: &[&OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ciq-asm"))
        .args(args)
        .output()
        .unwrap()
}
fn check(args: &[&OsStr]) {
    let output = run(args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn cli_signs_template_and_project_verifies_and_refuses_invalid_options_and_overwrite() {
    let temp = Temp(std::env::temp_dir().join(format!(
            "ciq-sign-cli-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    fs::create_dir(&temp.0).unwrap();
    let key = temp.0.join("key.der");
    let private = RsaPrivateKey::new(&mut OsRng, 4096).unwrap();
    fs::write(&key, private.to_pkcs8_der().unwrap().as_bytes()).unwrap();
    let input = temp.0.join("input.prg");
    let signed = temp.0.join("signed.prg");
    let original = Prg::from_sections(vec![
        Section::new(CODE, vec![0x38, 41, 0x16]).unwrap(),
        Section::new(0x12345678, vec![9, 8]).unwrap(),
    ]);
    fs::write(&input, original.to_bytes()).unwrap();
    let args = [
        OsStr::new("sign"),
        input.as_os_str(),
        signed.as_os_str(),
        OsStr::new("-y"),
        key.as_os_str(),
    ];
    check(&args);
    check(&[
        "verify".as_ref(),
        signed.as_os_str(),
        "-y".as_ref(),
        key.as_os_str(),
    ]);
    let preserved = fs::read(&signed).unwrap();
    assert!(!run(&args).status.success());
    assert_eq!(fs::read(&signed).unwrap(), preserved);
    assert!(
        !run(&["verify".as_ref(), signed.as_os_str(), "-y".as_ref()])
            .status
            .success()
    );
    assert!(
        !run(&[
            "verify".as_ref(),
            signed.as_os_str(),
            "-y".as_ref(),
            key.as_os_str(),
            "-y".as_ref(),
            key.as_os_str()
        ])
        .status
        .success()
    );
    let code = temp.0.join("code.mcasm");
    fs::write(&code, "ipush1 0x2a\nreturn\n").unwrap();
    let template_output = temp.0.join("template.prg");
    check(&[
        "-y".as_ref(),
        key.as_os_str(),
        input.as_os_str(),
        code.as_os_str(),
        template_output.as_os_str(),
    ]);
    check(&["verify".as_ref(), template_output.as_os_str()]);
    let changed = Prg::parse(&fs::read(&template_output).unwrap()).unwrap();
    assert_eq!(
        changed.unique_section(CODE).unwrap().payload(),
        &[0x38, 42, 0x16]
    );
    assert_eq!(changed.sections()[1], original.sections()[1]);
    let project = temp.0.join("project");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("sections.ciq"),
        "ciq-project 1\nsection 0xc0debabe 3 assembly code.mcasm\n",
    )
    .unwrap();
    fs::copy(&code, project.join("code.mcasm")).unwrap();
    let project_output = temp.0.join("project.prg");
    check(&[
        "project".as_ref(),
        project.as_os_str(),
        project_output.as_os_str(),
        "-y".as_ref(),
        key.as_os_str(),
    ]);
    check(&[
        "verify".as_ref(),
        project_output.as_os_str(),
        "-y".as_ref(),
        key.as_os_str(),
    ]);
    let invalid_output = temp.0.join("invalid.bin");
    assert!(
        !run(&[
            "data".as_ref(),
            code.as_os_str(),
            invalid_output.as_os_str(),
            "-y".as_ref(),
            key.as_os_str()
        ])
        .status
        .success()
    );
    assert!(!invalid_output.exists());
    let invalid_key = temp.0.join("invalid-key.der");
    fs::write(&invalid_key, b"not a key").unwrap();
    assert!(
        !run(&[
            "sign".as_ref(),
            input.as_os_str(),
            invalid_output.as_os_str(),
            "-y".as_ref(),
            invalid_key.as_os_str()
        ])
        .status
        .success()
    );
    assert!(!invalid_output.exists());
    let mut corrupt = changed.clone();
    corrupt
        .replace_same_length(CODE, &[0x38, 43, 0x16])
        .unwrap();
    let corrupt_path = temp.0.join("corrupt.prg");
    fs::write(&corrupt_path, corrupt.to_bytes()).unwrap();
    assert!(
        !run(&["verify".as_ref(), corrupt_path.as_os_str()])
            .status
            .success()
    );
}
