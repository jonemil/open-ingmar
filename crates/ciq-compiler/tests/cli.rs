use std::{fs, process::Command};

#[test]
fn cli_refuses_overwrite_and_creates_no_output_for_invalid_input() {
    let dir = std::env::temp_dir().join(format!("ciq-compiler-cli-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let source = dir.join("probe.mc");
    let output = dir.join("probe.mcasm");
    fs::write(&source, "function probe() { return 41; }").unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
            .arg(&source)
            .arg(&output)
            .output()
            .unwrap()
    };
    assert!(run().status.success());
    let saved = fs::read(&output).unwrap();
    assert!(!run().status.success());
    assert_eq!(fs::read(&output).unwrap(), saved);
    fs::remove_file(&output).unwrap();
    fs::write(&source, "function probe(x) { return 41; }").unwrap();
    let failed = run();
    assert!(!failed.status.success());
    assert!(!output.exists());
    let diagnostic = String::from_utf8(failed.stderr).unwrap();
    assert!(diagnostic.contains("probe.mc:1:16:"));
    fs::remove_dir_all(dir).unwrap();
}
