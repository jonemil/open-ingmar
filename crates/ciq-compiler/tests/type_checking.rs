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
fn partial_native_unions_cross_runtime_boundaries_without_erasure() {
    let cases = [
        "function f(x as Toybox.Lang.Number or Toybox.Lang.String) as Toybox.Lang.Number {return x;}",
        "function take(x as Toybox.Lang.Number) as Toybox.Lang.Number {return x;} function f(x as Toybox.Lang.Number or Toybox.Lang.String) as Toybox.Lang.Number {return take(x);}",
        "module M {function take(x as Toybox.Lang.Number) as Void {}} function f(x as Toybox.Lang.Number or Toybox.Lang.String) as Void {M.take(x);}",
        "class C {function take(x as Toybox.Lang.Number) as Void {} function f(x as Toybox.Lang.Number or Toybox.Lang.String) as Void {self.take(x);}}",
        "class C {function take(x as Toybox.Lang.Number) as Void {}} function f(c as C,x as Toybox.Lang.Number or Toybox.Lang.String) as Void {c.take(x);}",
        "class C {function initialize(x as Toybox.Lang.Number){}} function f(x as Toybox.Lang.Number or Toybox.Lang.String) as C {return new C(x);}",
        "function f(n as Toybox.Lang.Number,x as Toybox.Lang.Number or Toybox.Lang.String) as Toybox.Lang.Number {n=x;return n;}",
        "class C {var n=0;function f(x as Toybox.Lang.Number or Toybox.Lang.String) as Void {self.n=x;}}",
        "class C {var n=0;} function f(c as C,x as Toybox.Lang.Number or Toybox.Lang.String) as Void {c.n=x;}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>,x as Toybox.Lang.Number or Toybox.Lang.String) as Void {a[0]=x;}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>,x as Toybox.Lang.Number or Toybox.Lang.String) as Void {a.add(x);}",
        "function f(x as Toybox.Lang.Number or Toybox.Lang.String) as Toybox.Lang.String {return \"abc\".substring(x,null);}",
    ];
    for source in cases {
        let report = compile_sources_with_options(&[source], options(TypeCheckLevel::Informative))
            .unwrap_or_else(|e| panic!("{source}: {e}"));
        assert!(
            report
                .warnings
                .iter()
                .any(|d| d.kind == TypeDiagnosticKind::AmbiguousBoundary),
            "{source}"
        );
        assert!(
            report.artifact.to_ir_text().contains("union("),
            "actual union was erased: {source}"
        );
        assert!(!report.artifact.to_ir_text().contains(".cast "));
        common::pipeline(report.artifact);
        assert!(
            matches!(
                compile_sources_with_options(&[source], options(TypeCheckLevel::Strict)),
                Err(CompileError::TypeChecking(_))
            ),
            "{source}"
        );
        assert!(
            compile_sources_with_options(&[source], options(TypeCheckLevel::Gradual))
                .unwrap()
                .warnings
                .is_empty()
        );
    }
}

#[test]
fn partial_match_application_preserves_declared_results_and_complete_prg() {
    let source = include_str!("../../../examples/compiler-partial-matches/app.mc");
    let mut settings = options(TypeCheckLevel::Informative);
    settings.api = Some(ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap());
    let report = compile_sources_with_options(&[source], settings).unwrap();
    assert!(
        report
            .warnings
            .iter()
            .all(|d| d.kind == TypeDiagnosticKind::AmbiguousBoundary)
    );
    assert!(report.warnings.len() >= 8);
    common::application(
        &report.artifact,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x67912fb670174936b4f1cb9c9de3fd41u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    assert!(matches!(
        compile_sources_with_options(&[source], options(TypeCheckLevel::Strict)),
        Err(CompileError::TypeChecking(_))
    ));
}

#[test]
fn partial_matches_do_not_authorize_disjoint_contracts_or_nominal_covariance() {
    for source in [
        "function f(x as Toybox.Lang.Number or Toybox.Lang.String) as Toybox.Lang.Boolean {return x;}",
        "function f(x as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Array<Toybox.Lang.Object> {return x;}",
        "function f(x as Toybox.Lang.Dictionary<Toybox.Lang.Number,Toybox.Lang.Number or Toybox.Lang.String>) as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.Number> {return x;}",
        "class A {} class B {} function f(x as A or B) as A {return x;}",
        "function f(x as Toybox.Lang.Number or Toybox.Lang.String) as Toybox.Lang.Number {return x.length();}",
    ] {
        for level in [
            TypeCheckLevel::Gradual,
            TypeCheckLevel::Informative,
            TypeCheckLevel::Strict,
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
    let source = "function f(x as Toybox.Lang.Number or Null) as Toybox.Lang.Number {return x;}";
    assert!(compile_sources_with_options(&[source], options(TypeCheckLevel::Gradual)).is_ok());
    assert!(matches!(
        compile_sources_with_options(&[source], options(TypeCheckLevel::Strict)),
        Err(CompileError::TypeChecking(_))
    ));
}

#[test]
fn overlapping_container_parameters_remain_dynamic_in_both_directions() {
    let cases = [
        "using Toybox.Lang; function f(x as Lang.Array<Lang.Number or Lang.String>) as Lang.Array<Lang.Number> {return x;}",
        "using Toybox.Lang; function f(x as Lang.Array<Lang.Number>) as Lang.Array<Lang.Number or Lang.String> {return x;}",
        "using Toybox.Lang; function f(x as Lang.Array<Lang.Array<Lang.Number or Lang.String>>) as Lang.Array<Lang.Array<Lang.Number>> {return x;}",
        "using Toybox.Lang; function f(x as Lang.Dictionary<Lang.String,Lang.Integer>) as Lang.Dictionary<Lang.String,Lang.Number> {return x;}",
        "using Toybox.Lang; function f(x as Lang.Dictionary<Lang.Number,Lang.String>) as Lang.Dictionary<Lang.Integer,Lang.String> {return x;}",
        "using Toybox.Lang; function take(x as Lang.Array<Lang.Number>) as Void {} function f(x as Lang.Array<Lang.Number or Lang.String>) as Void {take(x);}",
        "using Toybox.Lang; class C {var values as Lang.Array<Lang.Number>;function initialize(x as Lang.Array<Lang.Number or Lang.String>){self.values=x;}}",
        "using Toybox.Lang; function f(a as Lang.Array<Lang.Number>,x as Lang.Array<Lang.Number or Lang.String>) as Void {a=x;}",
    ];
    for source in cases {
        let report = compile_sources_with_options(&[source], options(TypeCheckLevel::Informative))
            .unwrap_or_else(|e| panic!("{source}: {e}"));
        assert!(
            report
                .warnings
                .iter()
                .any(|d| d.kind == TypeDiagnosticKind::AmbiguousBoundary),
            "{source}"
        );
        assert!(!report.artifact.to_ir_text().contains(".cast "));
        common::pipeline(report.artifact);
        assert!(compile_sources_with_options(&[source], options(TypeCheckLevel::Gradual)).is_ok());
        assert!(
            matches!(
                compile_sources_with_options(&[source], options(TypeCheckLevel::Strict)),
                Err(CompileError::TypeChecking(_))
            ),
            "{source}"
        );
    }
}

#[test]
fn structural_signatures_and_disjoint_nested_parameters_remain_invariant() {
    for source in [
        "using Toybox.Lang; typedef A as interface {function read() as Lang.Number;}; typedef B as interface {function read() as Lang.Number or Lang.String;}; function f(x as B) as A {return x;}",
        "using Toybox.Lang; function f(x as interface {var value as Lang.Number or Lang.String;}) as interface {var value as Lang.Number;} {return x;}",
        "using Toybox.Lang; function f(x as Lang.Array<Lang.Array<Lang.Number>>) as Lang.Array<Lang.Array<Lang.Object>> {return x;}",
        "using Toybox.Lang; function f(x as Lang.Dictionary<Lang.Number,Lang.Integer>) as Lang.Dictionary<Lang.String,Lang.Number> {return x;}",
    ] {
        for level in [
            TypeCheckLevel::Gradual,
            TypeCheckLevel::Informative,
            TypeCheckLevel::Strict,
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
fn field_stores_and_constructors_report_unknown_values_at_the_write_or_call() {
    let cases = [
        "class Box {var value=0;function set(x){self.value=x;}}",
        "class Box {var value=0;} function f(b as Box,x){b.value=x;}",
        "typedef T as interface {var value as Toybox.Lang.Number;}; function f(b as T,x){b.value=x;}",
        "class Box {var value=0;function initialize(x as Toybox.Lang.Number){self.value=x;}} function f(x){return new Box(x);}",
    ];
    for source in cases {
        let report = compile_sources_with_options(&[source], options(TypeCheckLevel::Informative))
            .unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(
            report
                .warnings
                .iter()
                .filter(|d| d.kind == TypeDiagnosticKind::AmbiguousBoundary)
                .count(),
            1,
            "{source}"
        );
        common::pipeline(report.artifact);
        assert!(
            matches!(
                compile_sources_with_options(&[source], options(TypeCheckLevel::Strict)),
                Err(CompileError::TypeChecking(_))
            ),
            "{source}"
        );
    }
    // A member store needs no declaring unit when its receiver carries the shape.
    let fragment = "function f(b as interface {var value as Toybox.Lang.Number;},x){b.value=x;}";
    let report =
        compile_function_with_options(fragment, options(TypeCheckLevel::Informative)).unwrap();
    assert!(
        report
            .warnings
            .iter()
            .any(|d| d.kind == TypeDiagnosticKind::AmbiguousBoundary)
    );
}

#[test]
fn union_receivers_check_every_distinct_argument_contract() {
    let source = "class A {function put(x as Toybox.Lang.Number) as Void {}} class B {function put(x as Toybox.Lang.String) as Void {}} function f(receiver as A or B,x) as Void {receiver.put(x);}";
    let report =
        compile_sources_with_options(&[source], options(TypeCheckLevel::Informative)).unwrap();
    let boundaries: Vec<_> = report
        .warnings
        .iter()
        .filter(|d| d.kind == TypeDiagnosticKind::AmbiguousBoundary)
        .collect();
    assert_eq!(boundaries.len(), 2);
    assert!(
        boundaries
            .iter()
            .any(|d| d.location.diagnostic.message.contains("Number"))
    );
    assert!(
        boundaries
            .iter()
            .any(|d| d.location.diagnostic.message.contains("String"))
    );
    common::pipeline(report.artifact);
    let shared = source.replace("Toybox.Lang.String", "Toybox.Lang.Number");
    let report =
        compile_sources_with_options(&[&shared], options(TypeCheckLevel::Informative)).unwrap();
    assert_eq!(
        report
            .warnings
            .iter()
            .filter(|d| d.kind == TypeDiagnosticKind::AmbiguousBoundary)
            .count(),
        1
    );
}

#[test]
fn exact_interface_witnesses_and_typed_stores_remain_proven() {
    let source = "typedef T as interface {var value as Toybox.Lang.Number;function read() as Toybox.Lang.Number;}; class Box {var value=0;function set(x as Toybox.Lang.Number) as Void {self.value=x;} function read() as Toybox.Lang.Number {return self.value;}} function pass(x as Box) as T {return x;} function f(b as T,x as Toybox.Lang.Number) as Toybox.Lang.Number {b.value=x;return b.read();}";
    let report = compile_sources_with_options(&[source], options(TypeCheckLevel::Strict)).unwrap();
    assert!(report.warnings.is_empty());
    common::pipeline(report.artifact);
    let bad = source.replace("var value=0;", "var value as Toybox.Lang.String;");
    for level in [
        TypeCheckLevel::Gradual,
        TypeCheckLevel::Informative,
        TypeCheckLevel::Strict,
    ] {
        assert!(matches!(
            compile_sources_with_options(&[&bad], options(level)),
            Err(CompileError::Source(_))
        ));
    }
}

#[test]
fn interface_application_keeps_proven_source_witnesses_through_all_stages() {
    let source = include_str!("../../../examples/compiler-interfaces/app.mc");
    let report = compile_sources_with_options(&[source], options(TypeCheckLevel::Strict)).unwrap();
    assert!(report.warnings.is_empty());
    let app = ciq_compiler::application::ConsoleApplication {
        id: [1; 16],
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
