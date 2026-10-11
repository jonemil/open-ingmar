use std::{fs, process::Command};

#[test]
fn cli_api_context_checks_source_and_ir_and_rejects_invalid_options() {
    let dir = std::env::temp_dir().join(format!("ciq-compiler-api-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let source = dir.join("probe.mc");
    let output = dir.join("probe.ciqir");
    let run = |options: &[&str], input: &std::path::Path, output: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
            .args(options)
            .arg(input)
            .arg(output)
            .output()
            .unwrap()
    };
    fs::write(&source, "function show(n) { System.println(n.toChar()); }").unwrap();
    let failure = run(
        &["--target-api", "1.0.0", "--min-api", "1.0.0"],
        &source,
        &output,
    );
    assert!(String::from_utf8_lossy(&failure.stderr).contains("unavailable on target API"));
    assert!(!failure.status.success());
    assert!(!output.exists());
    fs::write(
        &source,
        "function show(n) { System.println(n.toString()); }",
    )
    .unwrap();
    let success = run(
        &[
            "--target-api",
            "1.0.0",
            "--min-api",
            "1.0.0",
            "--emit",
            "ir",
        ],
        &source,
        &output,
    );
    assert!(
        success.status.success(),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let resumed = dir.join("resumed.mcasm");
    let success = run(
        &[
            "--from",
            "ir",
            "--target-api",
            "1.0.0",
            "--min-api",
            "1.0.0",
        ],
        &output,
        &resumed,
    );
    assert!(success.status.success());
    let invalid = dir.join("invalid.mcasm");
    for (options, expected) in [
        (vec!["--target-api", "1.0.0"], "supplied together"),
        (vec!["--min-api", "1.0.0"], "supplied together"),
        (
            vec!["--target-api", "1.0", "--min-api", "1.0.0"],
            "major.minor.patch",
        ),
        (
            vec!["--target-api", "1.0.0", "--min-api", "1.3.0"],
            "exceeds target API",
        ),
        (
            vec![
                "--target-api",
                "1.0.0",
                "--target-api",
                "1.0.0",
                "--min-api",
                "1.0.0",
            ],
            "duplicate",
        ),
        (
            vec![
                "--from",
                "assembly",
                "--target-api",
                "1.0.0",
                "--min-api",
                "1.0.0",
            ],
            "assembly has no API contracts",
        ),
    ] {
        let failure = run(&options, &source, &invalid);
        assert!(!failure.status.success());
        assert!(
            String::from_utf8_lossy(&failure.stderr).contains(expected),
            "{}",
            String::from_utf8_lossy(&failure.stderr)
        );
        assert!(!invalid.exists());
    }
    fs::remove_dir_all(dir).unwrap();
}

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
    fs::write(&source, "function probe(x as Number) { return 41; }").unwrap();
    let failed = run();
    assert!(!failed.status.success());
    assert!(!output.exists());
    let diagnostic = String::from_utf8(failed.stderr).unwrap();
    assert!(diagnostic.contains("probe.mc:1:18:"));
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
        "ciq-ir\n.function probe\n.integer %0 42\n.return %1\n.end",
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

#[test]
fn cli_resumes_parameters_and_argument_lists() {
    let dir = std::env::temp_dir().join(format!("ciq-compiler-arguments-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let source = dir.join("probe.mc");
    let symbols = dir.join("symbols.mcasm");
    let ir = dir.join("probe.ciqir");
    let asm = dir.join("probe.mcasm");
    let direct = dir.join("direct.bin");
    let text = dir.join("text.bin");
    fs::write(
        &source,
        "function probe(first,second) { var x = Numbers.subtract(first,second); return x; }",
    )
    .unwrap();
    fs::write(
        &symbols,
        ".symbol m \"globals/Numbers\" id=0x02\n.symbol f \"subtract\" id=0x84\n",
    )
    .unwrap();
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
    for (from, input, output) in [("source", &source, &direct), ("assembly", &asm, &text)] {
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
    assert_eq!(fs::read(&direct).unwrap(), fs::read(text).unwrap());
    assert_eq!(&fs::read(&direct).unwrap()[..3], &[0x4a, 3, 1]);
    assert!(fs::read_to_string(ir).unwrap().contains("parameters=2"));
    fs::remove_dir_all(dir).unwrap();
}
#[test]
fn cli_binds_strings_across_saved_stages() {
    let dir = std::env::temp_dir().join(format!("ciq-compiler-strings-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let source = dir.join("probe.mc");
    let ir = dir.join("probe.ciqir");
    let asm = dir.join("probe.mcasm");
    let symbols = dir.join("symbols.mcasm");
    let data = dir.join("data.mcasm");
    fs::write(
        &source,
        "function probe() { System.println(\"hello #\"); return 42; }",
    )
    .unwrap();
    fs::write(
        &symbols,
        ".symbol system \"Toybox_System\" id=0x00800005\n.symbol print \"println\" id=0x00800067\n",
    )
    .unwrap();
    fs::write(
        &data,
        ".bytes at=0x00000000 aabb\n.string actual \"hello #\"\n",
    )
    .unwrap();
    for (from, emit, input, output) in [
        ("source", "ir", &source, &ir),
        ("ir", "assembly", &ir, &asm),
    ] {
        let run = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
            .args(["--from", from, "--emit", emit])
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
    let mut results = Vec::new();
    for (from, input) in [("source", &source), ("ir", &ir), ("assembly", &asm)] {
        let output = dir.join(format!("{from}.bin"));
        let run = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
            .args(["--from", from, "--emit", "code", "--symbols"])
            .arg(&symbols)
            .arg("--data")
            .arg(&data)
            .arg(input)
            .arg(&output)
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        results.push(fs::read(output).unwrap());
    }
    assert_eq!(results[0], results[1]);
    assert_eq!(results[0], results[2]);
    let invalid = dir.join("invalid.bin");
    for options in [
        vec!["--emit", "code", "--symbols"],
        vec!["--emit", "assembly", "--symbols"],
    ] {
        let run = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
            .args(options)
            .arg(&symbols)
            .arg(&source)
            .arg(&invalid)
            .output()
            .unwrap();
        assert!(!run.status.success());
        assert!(!invalid.exists());
    }
    fs::write(&data, ".string other \"missing\"").unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
        .args(["--emit", "code", "--symbols"])
        .arg(&symbols)
        .arg("--data")
        .arg(&data)
        .arg(&source)
        .arg(&invalid)
        .output()
        .unwrap();
    assert!(!run.status.success());
    assert!(!invalid.exists());
    fs::remove_dir_all(dir).unwrap();
}
