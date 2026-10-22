use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources, compile_sources_with_api, parse_function_ir, parse_unit_ir,
    parse_unit_ir_with_api,
};

const APP: &str = include_str!("../../../examples/compiler-comparators/app.mc");
const ORDERING: &str = include_str!("../../../examples/compiler-comparators/ordering.mc");

fn context() -> ApiContext {
    ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap()
}

#[test]
fn structural_comparators_preserve_saved_stages_and_runtime_member_identity() {
    let unit = compile_sources_with_api(&[APP, ORDERING], context()).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(" sort %"));
    let resumed = parse_unit_ir_with_api(&text, context()).unwrap();
    assert_eq!(unit, resumed);
    let asm = unit.lower();
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    let options = ConsoleApplication {
        id: [0x97; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    let prg = build_console_application(&unit, &options).unwrap();
    assert_eq!(prg, build_console_application(&resumed, &options).unwrap());
    let records =
        ciq_prg::data::classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
            .unwrap();
    assert!(records.iter().any(|record| matches!(record,
        ciq_prg::data::classes::Record::Class(class)
        if class.fields.iter().any(|field| field.key >> 8 == 0x00800150 && field.kind == 6)
    )));
    for (ty, literal) in [
        ("Number", "[1]"),
        ("Long", "[1l]"),
        ("Float", "[1.0]"),
        ("Double", "[1.0d]"),
        ("Boolean", "[true]"),
        ("Char", "['a']"),
        ("String", "[\"a\"]"),
    ] {
        let source = format!("function f(c as Ordering) {{ var a={literal}; a.sort(c); }}");
        let unit = compile_sources(&[&source, ORDERING]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap(), "{ty}");
    }
}

#[test]
fn structural_signatures_require_exact_object_inputs_and_number_results() {
    let narrowed = "class C { function initialize() {} function compare(a as C, b as Toybox.Lang.Object) as Toybox.Lang.Number { return 0; } } function run(c as C) { var a=[1,2]; a.sort(c); }";
    assert!(compile_sources(&[narrowed]).is_err());
    let broad = ORDERING
        .replace(
            "a as Toybox.Lang.Object, b as Toybox.Lang.Object",
            "a as Toybox.Lang.Object or Null, b as Toybox.Lang.Object or Null",
        )
        .replace(
            "return self.direction * a.toString().compareTo(b.toString());",
            "return 0;",
        );
    assert!(compile_sources(&[APP, &broad]).is_err());
    for changed in [
        ORDERING.replace("function compare(", "function renamed("),
        ORDERING.replace("a as Toybox.Lang.Object", "a as Toybox.Lang.Number"),
        ORDERING.replace(", b as Toybox.Lang.Object", ""),
        ORDERING.replace(
            "b as Toybox.Lang.Object",
            "b as Toybox.Lang.Object, extra as Toybox.Lang.Object",
        ),
        ORDERING
            .replace(
                "as Toybox.Lang.Number {\n        self.count",
                "as Toybox.Lang.String {\n        self.count",
            )
            .replace(
                "return self.direction * a.toString().compareTo(b.toString());",
                "return \"wrong\";",
            ),
    ] {
        assert_ne!(ORDERING, changed);
        assert!(
            compile_sources(&[APP, &changed]).is_err(),
            "accepted {changed}"
        );
    }
    let text = compile_sources(&[APP, ORDERING]).unwrap().to_ir_text();
    let changed = text.replace(
        "parameter-types=object:object",
        "parameter-types=integer:object",
    );
    assert!(text != changed);
    assert!(parse_unit_ir(&changed).is_err());
    let old = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(4, 2, 0)).unwrap();
    assert!(compile_sources_with_api(&[APP, ORDERING], old).is_err());
    assert!(parse_unit_ir_with_api(&text, old).is_err());
}

#[test]
fn erased_references_and_forged_ir_do_not_establish_interface_members() {
    for ty in [
        "Toybox.Lang.Object",
        "Toybox.WatchUi.View",
        "Toybox.Lang.Method",
    ] {
        let source = format!(
            "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>,c as {ty}) {{ a.sort(c); }}"
        );
        assert!(compile_function(&source).is_err());
    }
    let source =
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>,c as Ordering) { a.sort(c); }";
    let unit = compile_sources(&[source, ORDERING]).unwrap();
    let text = unit.declarations()[0].function.to_ir_text();
    assert!(parse_function_ir(&text).is_ok()); // Class/member resolution remains a unit obligation.
    assert!(parse_function_ir(&text.replace("source(Ordering@WatchUi.View)", "view")).is_err());
    let incomplete = "class Ordering extends Toybox.WatchUi.View { function initialize(direction as Toybox.Lang.Number) { View.initialize(); } }";
    assert!(compile_sources(&[source, incomplete]).is_err());
}

#[test]
fn comparator_shutdown_effects_propagate_to_reference_initialization() {
    let ordering = ORDERING.replace(
        "return self.direction * a.toString().compareTo(b.toString());",
        "System.exit(); return 0;",
    );
    let before = r#"
class Holder extends Toybox.WatchUi.View {
    var target as Ordering;
    function initialize(c as Ordering) {
        View.initialize();
        var a=[1,2];
        a.sort(c);
        self.target=c;
    }
}
"#;
    let error = compile_sources(&[before, &ordering])
        .unwrap_err()
        .to_string();
    assert!(error.contains("shutdown"), "{error}");
    let after = before.replace(
        "a.sort(c);\n        self.target=c;",
        "self.target=c;\n        a.sort(c);",
    );
    assert!(compile_sources(&[&after, &ordering]).is_ok());
    let wrapper = "module Sorter { function sort(a as Toybox.Lang.Array<Toybox.Lang.Number>,c as Ordering) { a.sort(c); } }";
    let indirect = before.replace("a.sort(c);", "Sorter.sort(a,c);");
    assert!(
        compile_sources(&[&indirect, &ordering, wrapper])
            .unwrap_err()
            .to_string()
            .contains("shutdown")
    );
}
