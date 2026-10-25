use ciq_compiler::{
    CompileError, CompileOptions, Severity, TypeCheckLevel, TypeDiagnosticKind,
    api::{ApiContext, ApiLevel},
    compile_function_with_options, compile_sources, compile_sources_with_options,
};
mod common;
fn options(level: TypeCheckLevel) -> CompileOptions {
    CompileOptions {
        type_check: level,
        ..Default::default()
    }
}

#[test]
fn checked_levels_report_dynamic_ambiguity_without_changing_the_program() {
    let source = "function f(value) as Toybox.Lang.Number {return value.size();}";
    let plain = compile_sources(&[source]).unwrap();
    let gradual =
        compile_sources_with_options(&[source], options(TypeCheckLevel::Gradual)).unwrap();
    assert!(gradual.warnings.is_empty());
    assert_eq!(plain, gradual.artifact);
    let info =
        compile_sources_with_options(&[source], options(TypeCheckLevel::Informative)).unwrap();
    assert_eq!(plain, info.artifact);
    common::pipeline(info.artifact);
    assert!(
        info.warnings
            .iter()
            .all(|d| d.severity == Severity::Warning)
    );
    for kind in [
        TypeDiagnosticKind::UntypedParameter,
        TypeDiagnosticKind::AmbiguousOperation,
        TypeDiagnosticKind::AmbiguousBoundary,
    ] {
        assert!(info.warnings.iter().any(|d| d.kind == kind));
    }
    let CompileError::TypeChecking(errors) =
        compile_sources_with_options(&[source], options(TypeCheckLevel::Strict)).unwrap_err()
    else {
        panic!("expected policy failure")
    };
    assert_eq!(info.warnings.len(), errors.len());
    assert!(errors.iter().all(|d| d.severity == Severity::Error));
}

#[test]
fn source_origins_and_fragment_policy_errors_remain_distinct() {
    let first = "module Other {function unused() as Void {}}";
    let second = "function f(value) as Toybox.Lang.Number {\n return value;\n}";
    let report =
        compile_sources_with_options(&[first, second], options(TypeCheckLevel::Informative))
            .unwrap();
    assert!(report.warnings.iter().all(|d| d.location.source == 1));
    assert!(
        report
            .warnings
            .iter()
            .any(|d| d.location.diagnostic.line == 2)
    );
    assert!(compile_function_with_options(second, options(TypeCheckLevel::Strict)).is_err());
    for level in [
        TypeCheckLevel::Gradual,
        TypeCheckLevel::Informative,
        TypeCheckLevel::Strict,
    ] {
        for source in [
            "function f() as Toybox.Lang.Number {return \"wrong\";}",
            "function f(){return missing;}",
            "function f(){break;}",
        ] {
            assert!(
                matches!(
                    compile_sources_with_options(&[source], options(level)),
                    Err(CompileError::Source(_))
                ),
                "{source}"
            );
        }
    }
}

#[test]
fn typed_native_user_and_container_boundaries_report_any_inputs() {
    for source in [
        "function f(x) as Toybox.Lang.Array<Toybox.Lang.Number> {return [x];}",
        "function f(x) as {:x as Toybox.Lang.Number} {return {:x=>x};}",
        "function f(x) as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.Number> {return {\"x\"=>x};}",
        "function take(x as Toybox.Lang.Number) as Void {} function f(x) as Void {take(x);}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>,x) as Void {a.add(x);}",
        "class Box {function take(x as Toybox.Lang.Number) as Void {}} function f(x) as Void {new Box().take(x);}",
    ] {
        let report = compile_sources_with_options(&[source], options(TypeCheckLevel::Informative))
            .unwrap_or_else(|e| panic!("{source}\n{e}"));
        assert!(
            report
                .warnings
                .iter()
                .any(|d| d.kind == TypeDiagnosticKind::AmbiguousBoundary),
            "{source}"
        );
        common::pipeline(report.artifact);
    }
}

#[test]
fn strict_typed_application_preserves_native_and_saved_prg_output() {
    use ciq_compiler::application::ConsoleApplication;
    let source = include_str!("../../../examples/compiler-casts/app.mc");
    let mut settings = options(TypeCheckLevel::Strict);
    settings.api = Some(ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap());
    let report = compile_sources_with_options(&[source], settings).unwrap();
    assert!(report.warnings.is_empty());
    let app = ConsoleApplication {
        id: 0x91645fbd41764dba942e4de3adf060edu128.to_be_bytes(),
        entry: "MinimalConsoleApp.onStart".into(),
    };
    let checked = common::application(&report.artifact, &app);
    let default = ciq_compiler::application::build_console_application(
        &compile_sources(&[source]).unwrap(),
        &app,
    )
    .unwrap();
    assert_eq!(checked.to_bytes(), default.to_bytes());
}

#[test]
fn cli_selects_policy_and_does_not_create_output_on_strict_failure() {
    use std::{
        fs,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };
    let root = std::env::temp_dir().join(format!(
        "ciq-policy-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let input = root.join("input.mc");
    fs::write(
        &input,
        "function f(value) as Toybox.Lang.Number {return value;}",
    )
    .unwrap();
    for (mode, success) in [
        ("gradual", true),
        ("informative", true),
        ("strict", false),
        ("off", false),
    ] {
        let output = root.join(format!("{mode}.ir"));
        let result = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
            .args(["--unit", "--emit", "ir", "--type-check", mode])
            .arg(&input)
            .arg(&output)
            .output()
            .unwrap();
        assert_eq!(
            result.status.success(),
            success,
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(output.exists(), success);
        if mode == "informative" {
            assert!(String::from_utf8_lossy(&result.stderr).contains("warning[UntypedParameter]"));
        }
    }
    fs::remove_dir_all(root).unwrap();
}
