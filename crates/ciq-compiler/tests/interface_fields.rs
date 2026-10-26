#[path = "common/checking.rs"]
mod checking;

use ciq_compiler::{
    FieldKind, InterfaceType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_sources, compile_sources_with_api, parse_unit_ir, parse_unit_ir_with_api,
};

const APP: &str = include_str!("../../../examples/compiler-comparators/app.mc");
const ORDERING: &str = include_str!("../../../examples/compiler-comparators/ordering.mc");

#[test]
fn interface_fields_preserve_types_and_null_slots_across_saved_stages() {
    let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap();
    let unit = compile_sources_with_api(&[APP, ORDERING], context).unwrap();
    let holder = unit
        .classes()
        .iter()
        .find(|c| c.name == "InterfaceHolder")
        .unwrap();
    assert_eq!(
        holder
            .fields
            .iter()
            .map(|f| f.kind.clone())
            .collect::<Vec<_>>(),
        [
            FieldKind::Interface {
                interface: InterfaceType::Comparable,
                nullable: false
            },
            FieldKind::Interface {
                interface: InterfaceType::Comparator,
                nullable: false
            },
            FieldKind::Interface {
                interface: InterfaceType::Comparable,
                nullable: true
            },
            FieldKind::Interface {
                interface: InterfaceType::Comparator,
                nullable: true
            },
        ]
    );
    let text = unit.to_ir_text();
    assert!(text.contains(".field-interface primary comparable\n"));
    assert!(text.contains(".field-interface selectedOrder nullable-comparator\n"));
    let resumed = parse_unit_ir_with_api(&text, context).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [0x51; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    let prg = build_console_application(&unit, &options).unwrap();
    assert_eq!(prg, build_console_application(&resumed, &options).unwrap());
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    let records =
        ciq_prg::data::classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
            .unwrap();
    assert!(records.iter().any(|record| matches!(record,
        ciq_prg::data::classes::Record::Class(class)
        if class.fields.iter().filter(|f| f.kind == 0 && f.value == 0 && f.key & 0xff == 0).count() == 4
    )));
    for changed in [
        text.replace(
            ".field-interface primary comparable",
            ".field-interface primary comparator",
        ),
        text.replace(
            ".field-interface primary comparable",
            ".field-interface primary unknown",
        ),
        text.replace(
            ".field-interface maybe nullable-comparable",
            ".field-interface maybe comparable",
        ),
    ] {
        assert_ne!(text, changed);
        assert!(parse_unit_ir(&changed).is_err());
    }
}

#[test]
fn non_null_field_initialization_and_signature_proofs_are_required() {
    for body in [
        "",
        "var x=self.value; self.value=c;",
        "self.call(); self.value=c;",
        "var x=self; self.value=c;",
        "if(true) { self.value=c; } else { self.value=c; }",
        "self.value=null;",
        "System.exit(); self.value=c;",
        "var n=c.compareTo(1); self.value=c;",
    ] {
        let source = format!(
            "class Holder {{ var value as Toybox.Lang.Comparable; function initialize(c as Toybox.Lang.Comparable) {{ {body} }} function call() {{ return 0; }} }}"
        );
        assert!(compile_sources(&[&source]).is_err(), "{source}");
    }
    for source in [
        "class C { var value as Toybox.Lang.Comparable; function other() {} }",
        "class C { var value as Toybox.Lang.Comparable = null; function initialize() {} }",
        "class C { var value as Toybox.Lang.Comparable or Null = 1; function initialize() {} }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    for (interface, member, parameters) in [
        ("Comparable", "compareTo", "a as Toybox.Lang.Object"),
        (
            "Comparator",
            "compare",
            "a as Toybox.Lang.Object,b as Toybox.Lang.Object",
        ),
    ] {
        let target = format!(
            "class Target {{ function initialize() {{}} function {member}({parameters}) as Toybox.Lang.Number {{ return 0; }} }}"
        );
        for null in ["", " or Null"] {
            let holder = format!(
                "class Holder {{ var value as Toybox.Lang.{interface}{null}; function initialize() {{ self.value=new Target(); }} function replace() {{ self.value=new Target(); }} }}"
            );
            let unit = compile_sources(&[&holder, &target]).unwrap();
            let text = unit.to_ir_text();
            assert_eq!(unit, parse_unit_ir(&text).unwrap());
            let changed = text.replace("parameter-types=object", "parameter-types=integer");
            assert_ne!(text, changed);
            assert!(parse_unit_ir(&changed).is_err());
        }
    }
}

#[test]
fn nullable_field_reads_need_guarded_local_snapshots() {
    let declaration =
        "class Holder { var value as Toybox.Lang.Comparable or Null; function initialize() {}";
    for body in [
        "return self.value.compareTo(1);",
        "if(self.value!=null) { return self.value.compareTo(1); } else { return 0; }",
        "var value=self.value; if(value!=null) { value=null; return value.compareTo(1); } else { return 0; }",
    ] {
        let source = format!("{declaration} function f() {{ {body} }} }}");
        assert!(checking::compile_sources(&[&source]).is_err(), "{source}");
    }
    let source = format!(
        "{declaration} function f() {{ var value=self.value; self.value=null; if(value!=null) {{ return value.compareTo(1); }} else {{ return 0; }} }} }}"
    );
    let unit = compile_sources(&[&source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    for ty in [
        "Comparable",
        "Comparable or Null",
        "Comparator",
        "Comparator or Null",
    ] {
        for value in ["o", "null", "1"] {
            let source = format!(
                "class C {{ var value as Toybox.Lang.{ty}; function initialize(v as Toybox.Lang.{ty}) {{ self.value=v; }} function replace(o as Toybox.Lang.Object) {{ self.value={value}; }} }}"
            );
            let allowed = (value == "null" && ty.ends_with(" or Null"))
                || (value == "1" && ty.starts_with("Comparable"));
            assert_eq!(compile_sources(&[&source]).is_ok(), allowed, "{source}");
        }
    }
}

#[test]
fn unused_field_types_require_api_levels_and_appbase_fields_generate_null_slots() {
    let source =
        "class C { var value as Toybox.Lang.Comparable or Null; function initialize() {} }";
    let unit = compile_sources(&[source]).unwrap();
    for low in [
        ApiContext::new(ApiLevel(4, 2, 0), ApiLevel(4, 2, 0)).unwrap(),
        ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(4, 2, 0)).unwrap(),
    ] {
        assert!(unit.validate_api(low).is_err());
        assert!(compile_sources_with_api(&[source], low).is_err());
        assert!(parse_unit_ir_with_api(&unit.to_ir_text(), low).is_err());
    }
    let app = "class App extends Toybox.Application.AppBase { var primary as Toybox.Lang.Comparable; var optional as Toybox.Lang.Comparator or Null; function initialize() { AppBase.initialize(); self.primary=1; } function onStart(state) { System.println(self.primary.compareTo(1)); System.exit(); } }";
    let unit = compile_sources(&[app]).unwrap();
    let options = ConsoleApplication {
        id: [0x52; 16],
        entry: "App.onStart".into(),
    };
    let prg = build_console_application(&unit, &options).unwrap();
    let records =
        ciq_prg::data::classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
            .unwrap();
    let slots = records
        .iter()
        .filter_map(|record| match record {
            ciq_prg::data::classes::Record::Class(class) => Some(&class.fields),
            _ => None,
        })
        .flatten()
        .filter(|f| f.kind == 0 && f.value == 0 && f.key & 0xff == 0)
        .count();
    assert_eq!(slots, 3); // Toybox placeholder and two application fields.
}
