use ciq_compiler::{
    CompileError, CompileOptions, TypeCheckLevel, TypeDiagnosticKind,
    api::{ApiContext, ApiLevel},
    compile_sources_with_options,
};
mod common;
fn options(level: TypeCheckLevel) -> CompileOptions {
    CompileOptions {
        type_check: level,
        ..Default::default()
    }
}

#[test]
fn partial_operations_keep_operands_and_fail_strict_checking() {
    for body in [
        "function f(x as Mixed) as Lang.Number {return x-1;}",
        "function f(x as Mixed) as Lang.Number {return x*2;}",
        "function f(x as Mixed) as Lang.Number {return x/2;}",
        "function f(x as Mixed) as Lang.Number {return x%2;}",
        "function f(x as Mixed) as Lang.Number {return x&1;}",
        "function f(x as Mixed) as Lang.Number {return x|1;}",
        "function f(x as Mixed) as Lang.Number {return x^1;}",
        "function f(x as Mixed) as Lang.Number {return x<<1;}",
        "function f(x as Mixed) as Lang.Number {return x>>1;}",
        "function f(x as Mixed) as Lang.Number {return -x;}",
        "function f(x as Mixed) as Lang.String {return +x;}",
        "function f(x as Mixed) as Lang.Number {return ~x;}",
        "function f(x as Mixed) as Lang.Number {return !x;}",
        "function f(x as Mixed) as Lang.Number {++x;return x;}",
        "function f(x as Mixed) as Lang.Number {--x;return x;}",
        "function f(x as Mixed) as Lang.Boolean {return x<1;}",
        "function f(x as Mixed) as Lang.Boolean {return x>=1;}",
        "function f(a as Lang.Array<Lang.Number>,i as Mixed) as Lang.Number {return a[i];}",
        "function f(a as Lang.Array<Lang.Number>,i as Mixed) as Void {a[i]=7;}",
        "function f(a as Lang.ByteArray,i as Mixed) as Lang.Number {return a[i];}",
        "function f(a as [Lang.Number,Lang.String],i as Mixed) as Mixed {return a[i];}",
        "function f(x as Mixed) as Lang.Number {return x.length();}",
        "function f(x as Lang.String or Null) as Lang.Number {return x.length();}",
        "function f(x as Lang.Array<Lang.Number> or Lang.Number) as Void {x.add(1);}",
    ] {
        let source =
            format!("using Toybox.Lang; typedef Mixed as Lang.Number or Lang.String; {body}");
        let report = compile_sources_with_options(&[&source], options(TypeCheckLevel::Informative))
            .unwrap_or_else(|e| panic!("{body}: {e}"));
        assert!(
            report.warnings.iter().any(|d| matches!(
                d.kind,
                TypeDiagnosticKind::AmbiguousOperation | TypeDiagnosticKind::AmbiguousBoundary
            )),
            "{body}"
        );
        assert!(!report.artifact.to_ir_text().contains(".cast "));
        common::pipeline(report.artifact);
        assert!(
            compile_sources_with_options(&[&source], options(TypeCheckLevel::Gradual))
                .unwrap()
                .warnings
                .is_empty()
        );
        assert!(
            matches!(
                compile_sources_with_options(&[&source], options(TypeCheckLevel::Strict)),
                Err(CompileError::TypeChecking(_))
            ),
            "{body}"
        );
    }
}

#[test]
fn partial_operations_preserve_disjoint_arity_api_and_source_identity_errors() {
    for source in [
        "function f() {return true-1;}",
        "function f() {return ~1.5;}",
        "function f() {return \"x\"-1;}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) {return a[\"x\"];}",
        "function f(x as Toybox.Lang.Number or Toybox.Lang.Boolean) {return x.length();}",
        "function f(x as Toybox.Lang.Number or Toybox.Lang.String) {return x.length(1);}",
        "function f(x as Toybox.Lang.Array<Toybox.Lang.Number> or Toybox.Lang.Number) {return x.sort(null);}",
        "class A {} class B {} function f(x as B or Toybox.Lang.Number) as A {return +x;}",
        "class C {function length() as Toybox.Lang.String{return \"x\";}} function f(x as C or Toybox.Lang.String) {return x.length(1);}",
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
    let mut settings = options(TypeCheckLevel::Gradual);
    settings.api = Some(ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(3, 0, 0)).unwrap());
    assert!(matches!(
        compile_sources_with_options(
            &[
                "function f(x as Toybox.Lang.Number or Toybox.Lang.String) {return x.substring(0,null);}"
            ],
            settings
        ),
        Err(CompileError::Source(_))
    ));
}

#[test]
fn partial_operation_application_resumes_through_complete_prg_stages() {
    let source = include_str!("../../../examples/compiler-partial-operations/app.mc");
    let mut settings = options(TypeCheckLevel::Informative);
    settings.api = Some(ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap());
    let report = compile_sources_with_options(&[source], settings).unwrap();
    assert!(!report.warnings.is_empty());
    common::application(
        &report.artifact,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x2d97be77ab3d483e94eaf08d10489a9au128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
}
