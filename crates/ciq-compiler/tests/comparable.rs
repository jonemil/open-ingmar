use ciq_compiler::{
    api::{ApiContext, ApiLevel, Type},
    compile_sources, parse_unit_ir,
};

const APP: &str = include_str!("../../../examples/compiler-comparators/app.mc");
const ORDERING: &str = include_str!("../../../examples/compiler-comparators/ordering.mc");
const COMPARABLE: &str = "Toybox.Lang.Comparable";

fn context(target: ApiLevel, minimum: ApiLevel) -> ApiContext {
    ApiContext::new(target, minimum).unwrap()
}

#[test]
fn comparable_is_a_versioned_interface_without_a_runtime_class() {
    let catalogue = ciq_compiler::api::core();
    let declaration = catalogue.interface(COMPARABLE).unwrap();
    assert!(catalogue.class(COMPARABLE).is_none());
    assert_eq!(declaration.since, ApiLevel(5, 0, 0));
    let method = &declaration.methods[0];
    assert_eq!(method.name, "compareTo");
    assert_eq!(method.parameters, [Type::named("Toybox.Lang.Object")]);
    assert_eq!(method.result, Type::named("Toybox.Lang.Number"));
    assert_eq!(method.since, ApiLevel(5, 0, 0));
}

#[test]
fn source_signature_proofs_survive_saved_ir_and_require_api_levels() {
    let unit = compile_sources(&[APP, ORDERING]).unwrap();
    let current = context(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0));
    for candidate in [&unit, &parse_unit_ir(&unit.to_ir_text()).unwrap()] {
        candidate
            .check_interface("TextOrder", COMPARABLE, current)
            .unwrap();
        candidate
            .check_interface("Ordering", "Toybox.Lang.Comparator", current)
            .unwrap();
        assert!(
            candidate
                .check_interface("Ordering", COMPARABLE, current)
                .is_err()
        );
        assert!(
            candidate
                .check_interface("TextOrder", "Toybox.Lang.Comparator", current)
                .is_err()
        );
        for low in [
            context(ApiLevel(4, 2, 0), ApiLevel(4, 2, 0)),
            context(ApiLevel(5, 2, 0), ApiLevel(4, 2, 0)),
        ] {
            let error = candidate
                .check_interface("TextOrder", COMPARABLE, low)
                .unwrap_err();
            assert!(error.contains("requires API 5.0.0"), "{error}");
        }
        for (class, interface) in [
            ("Missing", COMPARABLE),
            ("Toybox.Lang.Number", COMPARABLE),
            ("TextOrder", "Toybox.Lang.Object"),
            ("TextOrder", "Missing"),
        ] {
            assert!(
                candidate
                    .check_interface(class, interface, current)
                    .is_err()
            );
        }
    }
}

#[test]
fn source_comparable_requires_the_exact_member_signature() {
    let current = context(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0));
    for (parameters, result, body, accepted) in [
        (
            "o as Toybox.Lang.Object",
            "Toybox.Lang.Number",
            "return 0;",
            true,
        ),
        (
            "o as Toybox.Lang.Object or Null",
            "Toybox.Lang.Number",
            "return 0;",
            false,
        ),
        (
            "o as Toybox.Lang.Number",
            "Toybox.Lang.Number",
            "return 0;",
            false,
        ),
        ("o as C", "Toybox.Lang.Number", "return 0;", false),
        ("", "Toybox.Lang.Number", "return 0;", false),
        (
            "a as Toybox.Lang.Object,b as Toybox.Lang.Object",
            "Toybox.Lang.Number",
            "return 0;",
            false,
        ),
        (
            "o as Toybox.Lang.Object",
            "Toybox.Lang.Number or Null",
            "return null;",
            false,
        ),
        (
            "o as Toybox.Lang.Object",
            "Toybox.Lang.Long",
            "return 0l;",
            false,
        ),
        ("o as Toybox.Lang.Object", "Void", "return;", false),
    ] {
        let source = format!(
            "class C {{ function initialize() {{}} function compareTo({parameters}) as {result} {{ {body} }} }}"
        );
        let unit = compile_sources(&[&source]).unwrap();
        let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
        for candidate in [&unit, &resumed] {
            assert_eq!(
                candidate.check_interface("C", COMPARABLE, current).is_ok(),
                accepted,
                "{source}"
            );
        }
    }
    let source = "class C { function initialize() {} function compareTo(o as Toybox.Lang.Object) as Toybox.Lang.Number { return 0; } }";
    let unit = compile_sources(&[source]).unwrap();
    let changed = unit
        .to_ir_text()
        .replace("parameter-types=object", "parameter-types=integer");
    assert_ne!(changed, unit.to_ir_text());
    let resumed = parse_unit_ir(&changed).unwrap();
    assert!(resumed.check_interface("C", COMPARABLE, current).is_err());
}

#[test]
fn declaration_does_not_enable_interface_tests_or_storage() {
    for source in [
        "import Toybox.Lang; class Comparable { function initialize() {} }",
        "import Toybox.Lang; function f(o as Object) { return o instanceof Comparable; }",
        "import Toybox.Lang; function f(o as Comparable) { return o; }",
        "import Toybox.Lang; function f(o as Object) { return o as Comparable; }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}
