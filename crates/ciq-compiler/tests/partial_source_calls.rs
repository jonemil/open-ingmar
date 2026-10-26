use ciq_compiler::{
    CompileError, CompileOptions, TypeCheckLevel, TypeDiagnosticKind, compile_sources,
    compile_sources_with_options, parse_unit_ir,
};
mod common;
const CLASSES: &str = "class A { function value() as Toybox.Lang.Number {return 41;} function length() as Toybox.Lang.String {return \"a\";} function effect() as Void {} } class B {}";

#[test]
fn partial_receivers_retain_all_identities_and_check_strict_ambiguity() {
    for body in [
        "function f(x as A or B) as Toybox.Lang.Number {return x.value();}",
        "function f(x as A or Null) as Toybox.Lang.Number {return x.value();}",
        "function f(x as A or Toybox.Lang.Number) as Toybox.Lang.Number {return x.value();}",
        "function f(x as A or B or Toybox.Lang.String or Null) {return x.length();}",
        "function f(x as A or B) as Void {x.effect();}",
        "function f(x as A or B) as Void {x.value();}",
    ] {
        let report = compile_sources_with_options(
            &[CLASSES, body],
            CompileOptions {
                type_check: TypeCheckLevel::Informative,
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("{body}: {e}"));
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.kind == TypeDiagnosticKind::AmbiguousOperation),
            "{body}"
        );
        assert!(!report.artifact.to_ir_text().contains(".cast "));
        common::pipeline(report.artifact);
        assert!(
            matches!(
                compile_sources_with_options(
                    &[CLASSES, body],
                    CompileOptions {
                        type_check: TypeCheckLevel::Strict,
                        ..Default::default()
                    }
                ),
                Err(CompileError::TypeChecking(_))
            ),
            "{body}"
        );
    }
}

#[test]
fn complete_mixed_lookup_joins_source_and_native_signatures() {
    let source = "function f(x as A or Toybox.Lang.String) as Toybox.Lang.Number or Toybox.Lang.String {return x.length();}";
    let report = compile_sources_with_options(
        &[CLASSES, source],
        CompileOptions {
            type_check: TypeCheckLevel::Strict,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(report.warnings.is_empty());
    common::pipeline(report.artifact);
    let guarded = "function f(x as A or Null) as Toybox.Lang.Number {if(x==null){return 0;}return x.value();}";
    let report = compile_sources_with_options(
        &[CLASSES, guarded],
        CompileOptions {
            type_check: TypeCheckLevel::Strict,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(report.warnings.is_empty());
    common::pipeline(report.artifact);
    let contextual = "class Receiver {function take(xs as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number {return xs.size();}} function f(x as Receiver or B) {return x.take([]);}";
    common::pipeline(compile_sources(&[CLASSES, contextual]).unwrap());
}

#[test]
fn lookup_checks_available_signatures_and_rejects_forged_metadata() {
    for source in [
        "function f(x as A or B) {return x.absent();}",
        "function f(x as A or B) {return x.value(1);}",
        "function f(x as A or B) {return x.effect();}",
        "function f(x as A or Toybox.Lang.String) {return x.length(1);}",
        "class V extends Toybox.WatchUi.View {function onUpdate(dc) {}} function f(x as A or V) {x.onUpdate(null);}",
    ] {
        assert!(compile_sources(&[CLASSES, source]).is_err(), "{source}");
    }
    let old_api = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(5, 2, 0),
        ciq_compiler::api::ApiLevel(3, 0, 0),
    )
    .unwrap();
    assert!(matches!(
        compile_sources_with_options(
            &[
                CLASSES,
                "function f(x as A or Toybox.Lang.String) {return x.substring(0,null);}"
            ],
            CompileOptions {
                api: Some(old_api),
                ..Default::default()
            }
        ),
        Err(CompileError::Source(_))
    ));
    let unit = compile_sources(&[CLASSES, "function f(x as A or B) {return x.value();}"]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(" A|B value"));
    assert!(parse_unit_ir(&text.replace(" A|B value", " A value")).is_err());
    assert!(parse_unit_ir(&text.replace(" A|B value", " A|Missing value")).is_err());
    let call = text
        .lines()
        .find(|line| line.starts_with(".call-object "))
        .unwrap();
    let forged = call.replace("result=any", "result=integer");
    assert_ne!(call, forged);
    assert!(parse_unit_ir(&text.replace(call, &forged)).is_err());
}

#[test]
fn complete_application_retains_source_and_native_dispatch() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-partial-operations/app.mc"
    )])
    .unwrap();
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x2d97be77ab3d483e94eaf08d10489a9au128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
}
