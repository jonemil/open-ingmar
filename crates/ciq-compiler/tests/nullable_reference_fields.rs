#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{FieldKind, compile_sources, parse_unit_ir};

const TOKEN: &str = "class Token { function initialize() {} function value() { return 41; } } class Other { function initialize() {} function value() { return 9; } }";

#[test]
fn nullable_fields_retain_snapshot_identity_on_every_source_owner_base() {
    for (base, initialize) in [
        ("", ""),
        ("extends Toybox.WatchUi.View", "View.initialize();"),
        (
            "extends Toybox.WatchUi.BehaviorDelegate",
            "BehaviorDelegate.initialize();",
        ),
        (
            "extends Toybox.Application.AppBase",
            "AppBase.initialize();",
        ),
    ] {
        let source = format!(
            r#"
            class Holder {base} {{
                var target as Token or Null;
                var other as Token or Null = null;
                function initialize() {{ {initialize} }}
                function clear() {{ self.target = null; }}
                function load() as Token or Null {{ return self.target; }}
                function replace(token as Token or Null) {{ self.target = token; }}
                function run() {{
                    self.target = new Token();
                    var snapshot = self.target;
                    self.clear();
                    if (snapshot != null) {{ return snapshot.value(); }}
                    return 0;
                }}
            }}
        "#
        );
        let unit = compile_sources(&[&source, TOKEN]).unwrap();
        assert!(matches!(
            unit.classes()[0].fields[0].kind,
            FieldKind::NullableSourceObject(_)
        ));
        let text = unit.to_ir_text();
        assert!(text.contains(".field-reference target nullable-source(Token@Lang.Object)"));
        let resumed = parse_unit_ir(&text).unwrap();
        assert_eq!(unit, resumed);
        assert_eq!(unit.lower(), resumed.lower());
        assert_eq!(
            unit.lower(),
            ciq_assembly::Program::parse(&unit.lower().to_text()).unwrap()
        );
    }
}

#[test]
fn source_fields_require_snapshot_proofs_and_exact_assignment_classes() {
    for body in [
        "function f() { return self.target.value(); }",
        "function f() { if (self.target != null) { return self.target.value(); } return 0; }",
        "function f() { self.target = new Other(); }",
        "function f(value as Toybox.Lang.Object) { self.target = value; }",
        "function f(value as Other or Null) { self.target = value; }",
        "function f() as Token { return self.target; }",
        "function f() { var snapshot = self.target; if (snapshot != null) { snapshot = null; return snapshot.value(); } return 0; }",
    ] {
        let source = format!(
            "class Holder {{ var target as Token or Null; function initialize() {{}} {body} }}"
        );
        assert!(
            checking::compile_sources(&[&source, TOKEN]).is_err(),
            "{body}"
        );
    }
    for field in [
        "var target as Token or Null = new Token();",
        "var target as Token or Null = (null);",
        "var target as Missing or Null;",
    ] {
        let source = format!(
            "class Holder extends Toybox.Application.AppBase {{ {field} function initialize() {{ AppBase.initialize(); }} }}"
        );
        assert!(
            checking::compile_sources(&[&source, TOKEN]).is_err(),
            "{field}"
        );
    }
}

#[test]
fn saved_unit_rechecks_field_type_identity_and_nullability() {
    let source = "class Holder { var target as Token or Null; function initialize() {} function load() as Token or Null { return self.target; } }";
    let text = compile_sources(&[source, TOKEN]).unwrap().to_ir_text();
    for changed in [
        text.replace(
            ".field-reference target nullable-source(Token@Lang.Object)",
            ".field-reference target source(Token@Lang.Object)",
        ),
        text.replace(
            ".field-reference target nullable-source(Token@Lang.Object)",
            ".field-reference target nullable-source(Token@WatchUi.View)",
        ),
        text.replace(
            ".field-reference target nullable-source(Token@Lang.Object)",
            ".field-reference target nullable-source(Other@Lang.Object)",
        ),
        text.replace(
            ".field-reference target nullable-source(Token@Lang.Object)",
            ".field-reference target nullable-object",
        ),
        text.replace(
            ".field-reference target nullable-source(Token@Lang.Object)",
            ".field-reference target Token@Lang.Object",
        ),
        text.replace(
            ".field-reference target nullable-source(Token@Lang.Object)",
            ".field-reference target nullable-source()",
        ),
    ] {
        assert_ne!(text, changed);
        assert!(parse_unit_ir(&changed).is_err(), "{changed}");
    }
}
