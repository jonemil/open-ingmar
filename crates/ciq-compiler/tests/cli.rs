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

#[test]
fn cli_can_stop_and_resume_each_pipeline_stage() {
    let dir = std::env::temp_dir().join(format!("ciq-compiler-pipeline-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let source = dir.join("probe.mc");
    fs::write(&source, "function probe() { return 42; }").unwrap();
    let run = |from: &str, emit: &str, input: &std::path::Path, output: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
            .args(["--from", from, "--emit", emit])
            .arg(input)
            .arg(output)
            .output()
            .unwrap()
    };
    let ir = dir.join("probe.ciqir");
    let asm = dir.join("probe.mcasm");
    let direct = dir.join("direct.bin");
    let via_ir = dir.join("via-ir.bin");
    let via_asm = dir.join("via-asm.bin");
    assert!(run("source", "ir", &source, &ir).status.success());
    assert!(run("ir", "assembly", &ir, &asm).status.success());
    assert!(run("source", "code", &source, &direct).status.success());
    assert!(run("ir", "code", &ir, &via_ir).status.success());
    assert!(run("assembly", "code", &asm, &via_asm).status.success());
    assert_eq!(fs::read(&direct).unwrap(), [0x35, 1, 0x38, 42, 0x16]);
    assert_eq!(fs::read(&direct).unwrap(), fs::read(&via_ir).unwrap());
    assert_eq!(fs::read(&direct).unwrap(), fs::read(&via_asm).unwrap());
    let invalid = dir.join("invalid.bin");
    fs::write(
        &ir,
        "ciq-ir 1\n.function probe\n.integer %0 42\n.return %1\n.end",
    )
    .unwrap();
    assert!(!run("ir", "code", &ir, &invalid).status.success());
    assert!(!invalid.exists());
    assert!(!run("assembly", "ir", &asm, &invalid).status.success());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn cli_resumes_saved_arithmetic_ir_and_assembly() {
    let dir = std::env::temp_dir().join(format!("ciq-compiler-arithmetic-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let source = dir.join("probe.mc");
    let ir = dir.join("probe.ciqir");
    let asm = dir.join("probe.mcasm");
    let direct = dir.join("native.bin");
    let text = dir.join("text.bin");
    fs::write(&source, "function probe() { return 40 + (8 - 5); }").unwrap();
    for (options, input, output) in [
        (vec!["--emit", "ir"], &source, &ir),
        (vec!["--from", "ir"], &ir, &asm),
        (vec!["--emit", "code"], &source, &direct),
        (vec!["--from", "assembly", "--emit", "code"], &asm, &text),
    ] {
        let run = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
            .args(options)
            .arg(input)
            .arg(output)
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
    assert_eq!(fs::read(direct).unwrap(), fs::read(text).unwrap());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn cli_resumes_local_declarations_and_reports_unknown_reads() {
    let dir = std::env::temp_dir().join(format!("ciq-compiler-locals-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let source = dir.join("probe.mc");
    let ir = dir.join("probe.ciqir");
    let asm = dir.join("probe.mcasm");
    let direct = dir.join("native.bin");
    let text = dir.join("text.bin");
    fs::write(
        &source,
        "function probe() { var x = 40 + 2; var y = x - 5; return x + y - x; }",
    )
    .unwrap();
    for (options, input, output) in [
        (vec!["--emit", "ir"], &source, &ir),
        (vec!["--from", "ir"], &ir, &asm),
        (vec!["--emit", "code"], &source, &direct),
        (vec!["--from", "assembly", "--emit", "code"], &asm, &text),
    ] {
        let run = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
            .args(options)
            .arg(input)
            .arg(output)
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
    assert_eq!(fs::read(&direct).unwrap(), fs::read(text).unwrap());
    let invalid = dir.join("invalid.bin");
    fs::write(&source, "function probe() { return missing; }").unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
        .args(["--emit", "code"])
        .arg(source)
        .arg(&invalid)
        .output()
        .unwrap();
    assert!(!run.status.success());
    assert!(!invalid.exists());
    assert!(String::from_utf8_lossy(&run.stderr).contains("not yet initialized"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn cli_binds_module_calls_in_native_ir_and_assembly_paths() {
    let dir = std::env::temp_dir().join(format!("ciq-compiler-calls-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let source = dir.join("probe.mc");
    let symbols = dir.join("symbols.mcasm");
    let ir = dir.join("probe.ciqir");
    let asm = dir.join("probe.mcasm");
    let direct = dir.join("native.bin");
    let resumed = dir.join("resumed.bin");
    let text = dir.join("text.bin");
    fs::write(
        &source,
        "function probe() { var x = Numbers.answer(); return x + x; }",
    )
    .unwrap();
    fs::write(&symbols, ".symbol field \"Numbers\" id=0x99\n.symbol module \"globals/Numbers\" id=0x02\n.symbol member \"answer\" id=0x84\n").unwrap();
    for (options, input, output) in [
        (vec!["--emit", "ir"], &source, &ir),
        (vec!["--from", "ir"], &ir, &asm),
    ] {
        let run = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
            .args(options)
            .arg(input)
            .arg(output)
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
    for (from, input, output) in [
        ("source", &source, &direct),
        ("ir", &ir, &resumed),
        ("assembly", &asm, &text),
    ] {
        let run = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
            .args(["--from", from, "--emit", "code", "--symbols"])
            .arg(&symbols)
            .arg(input)
            .arg(output)
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
    assert_eq!(fs::read(&direct).unwrap(), fs::read(resumed).unwrap());
    assert_eq!(fs::read(&direct).unwrap(), fs::read(text).unwrap());
    assert_eq!(
        &fs::read(&direct).unwrap()[3..13],
        &[0x45, 0, 0, 0, 2, 0, 0, 0, 0x84, 0x48]
    );
    // Missing binding and incompatible option combinations create no output.
    let invalid = dir.join("invalid.bin");
    fs::write(
        &symbols,
        ".symbol field \"Numbers\" id=0x99\n.symbol member \"answer\" id=0x84\n",
    )
    .unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
        .args(["--emit", "code", "--symbols"])
        .arg(&symbols)
        .arg(&source)
        .arg(&invalid)
        .output()
        .unwrap();
    assert!(!run.status.success());
    assert!(String::from_utf8_lossy(&run.stderr).contains("globals/Numbers"));
    assert!(!invalid.exists());
    let run = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
        .args(["--emit", "ir", "--symbols"])
        .arg(&symbols)
        .arg(&source)
        .arg(&invalid)
        .output()
        .unwrap();
    assert!(!run.status.success());
    assert!(!invalid.exists());
    fs::remove_dir_all(dir).unwrap();
}
