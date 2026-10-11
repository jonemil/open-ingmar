use ciq_compiler::{
    api::{ApiContext, ApiLevel, core},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources_with_api, compile_with_api, parse_ir_with_api, parse_unit_ir_with_api,
};

fn context(target: &str, minimum: &str) -> ApiContext {
    ApiContext::new(target.parse().unwrap(), minimum.parse().unwrap()).unwrap()
}

#[test]
fn context_requires_explicit_consistent_levels() {
    for invalid in [
        "1",
        "1.3",
        "1.3.0.0",
        "1..0",
        "+1.3.0",
        "1.3.0 ",
        "65536.0.0",
    ] {
        assert!(invalid.parse::<ApiLevel>().is_err(), "{invalid}");
    }
    assert_eq!("1.3.0".parse::<ApiLevel>().unwrap().to_string(), "1.3.0");
    assert!(ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 3, 0)).is_err());
    assert!(ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(0, 9, 0)).is_err());
    let explicit = context("5.0.0", "1.0.0");
    assert_eq!(explicit.target(), ApiLevel(5, 0, 0));
    assert_eq!(explicit.minimum(), ApiLevel(1, 0, 0));
}

#[test]
fn availability_errors_precede_argument_validation_in_source_and_ir() {
    let source = "function show(n) { System.println(n.toChar(1)); }";
    let text = "ciq-ir\n.function show parameters=1\n.parameter %0 0\n.integer %1 1\n.call-native %2 %0 toChar %1\n.println %2\n.return-void\n.end\n";
    for (target, minimum, expected) in [
        ("1.2.9", "1.0.0", "unavailable on target API 1.2.9"),
        ("1.3.0", "1.0.0", "application minimum API is 1.0.0"),
        ("1.3.0", "1.3.0", "argument count"),
    ] {
        let context = context(target, minimum);
        let source_error = compile_with_api(source, context).unwrap_err();
        let ir_error = parse_ir_with_api(text, context).unwrap_err();
        assert!(source_error.message.contains(expected), "{source_error}");
        assert_eq!(source_error.message, ir_error.message);
        assert!(source_error.offset > 0);
        assert_eq!(ir_error.line, 5);
    }
    assert!(
        compile(source)
            .unwrap_err()
            .message
            .contains("argument count")
    );
    let unknown = compile_with_api(
        "function show(n) { System.println(n.missing()); }",
        context("1.0.0", "1.0.0"),
    )
    .unwrap_err();
    assert!(unknown.message.contains("not known"));
    assert!(!unknown.message.contains("unavailable"));
}

#[test]
fn catalogue_checks_later_members_and_inherited_owners() {
    let early = context("4.9.9", "1.0.0");
    assert!(
        early
            .method(core(), "Toybox.Lang.Number", "compareTo")
            .unwrap_err()
            .contains("requires API 5.0.0")
    );
    let initial = context("1.0.0", "1.0.0");
    assert_eq!(
        initial
            .method(core(), "Toybox.Lang.Number", "toString")
            .unwrap()
            .declaring_class
            .name,
        "Toybox.Lang.Object"
    );
    assert!(initial.function(core(), "Toybox.System", "println").is_ok());
    assert!(
        initial
            .check_class(core(), "Missing.Class")
            .unwrap_err()
            .contains("not known")
    );
    assert!(
        initial
            .function(core(), "Toybox.System", "missing")
            .unwrap_err()
            .contains("not known")
    );
}

#[test]
fn explicit_context_preserves_native_text_and_complete_application_paths() {
    let context = context("1.0.0", "1.0.0");
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-native-calls/app.mc"),
            include_str!("../../../examples/compiler-native-calls/numbers.mc"),
        ],
        context,
    )
    .unwrap();
    unit.validate_api(context).unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap();
    let options = ConsoleApplication {
        id: [9; 16],
        entry: "start".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
    let function = compile_with_api(
        "function show(n) { System.println(n.format(\"%02d\")); }",
        context,
    )
    .unwrap();
    function.validate_api(context).unwrap();
    assert_eq!(
        function,
        parse_ir_with_api(&function.to_ir_text(), context).unwrap()
    );
    let lifecycle = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-lifecycle/app.mc"),
            include_str!("../../../examples/compiler-lifecycle/helpers.mc"),
        ],
        context,
    )
    .unwrap();
    lifecycle.validate_api(context).unwrap();
    assert_eq!(
        lifecycle,
        parse_unit_ir_with_api(&lifecycle.to_ir_text(), context).unwrap()
    );
}

#[test]
fn unit_source_and_text_report_availability_at_the_offending_body() {
    let sources = [
        "module A { function ok() { return 1; } }",
        "module B { function show(n) { System.println(n.toChar()); } }",
    ];
    let context = context("1.0.0", "1.0.0");
    let error = compile_sources_with_api(&sources, context).unwrap_err();
    assert_eq!(error.source, 1);
    assert!(error.diagnostic.message.contains("unavailable on target"));
    let text = "ciq-unit\n.owner global\nciq-ir\n.function show parameters=1\n.parameter %0 0\n.call-native %1 %0 toChar\n.println %1\n.return-void\n.end\n";
    let error = parse_unit_ir_with_api(text, context).unwrap_err();
    assert_eq!(error.line, 6);
    assert!(error.message.contains("unavailable on target"));
}
