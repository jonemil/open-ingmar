#[path = "common/checking.rs"]
mod checking;
use ciq_assembly::Program;
use ciq_compiler::{
    FieldKind, ScalarFieldType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_sources, compile_sources_with_api, parse_unit_ir, parse_unit_ir_with_api,
};

const SCALARS: &[(&str, &str, ScalarFieldType)] = &[
    ("Number", "integer", ScalarFieldType::Number),
    ("Long", "long", ScalarFieldType::Long),
    ("Float", "float", ScalarFieldType::Float),
    ("Double", "double", ScalarFieldType::Double),
    ("Boolean", "boolean", ScalarFieldType::Boolean),
    ("Char", "char", ScalarFieldType::Char),
    ("String", "string", ScalarFieldType::String),
];

#[test]
fn scalar_fields_preserve_complete_saved_stages_and_null_metadata() {
    let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(2, 1, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[include_str!(
            "../../../examples/compiler-scalar-fields/app.mc"
        )],
        context,
    )
    .unwrap();
    let text = unit.to_ir_text();
    let resumed = parse_unit_ir_with_api(&text, context).unwrap();
    assert_eq!(unit, resumed);
    for (_, tag, ty) in SCALARS {
        for nullable in [false, true] {
            assert!(
                unit.classes()[0]
                    .fields
                    .iter()
                    .any(|field| field.kind == FieldKind::Scalar { ty: *ty, nullable })
            );
            assert!(text.contains(&format!(
                " {}{tag}\n",
                if nullable { "nullable-" } else { "" }
            )));
        }
    }
    let assembly = unit.lower();
    assert_eq!(assembly, Program::parse(&assembly.to_text()).unwrap());
    let options = ConsoleApplication {
        id: [101; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    let prg = build_console_application(&unit, &options).unwrap();
    assert_eq!(prg, build_console_application(&resumed, &options).unwrap());
    let records =
        ciq_prg::data::classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
            .unwrap();
    let slots = records
        .iter()
        .flat_map(|record| match record {
            ciq_prg::data::classes::Record::Class(class) => class.fields.as_slice(),
            _ => &[],
        })
        .filter(|field| field.kind == 0 && field.symbol_id() < 0x0080_0000)
        .collect::<Vec<_>>();
    assert_eq!(slots.len(), 15);
    assert!(slots.iter().all(|slot| slot.value == 0));
    assert!(assembly.to_text().contains("0x80000000")); // Float negative zero.
}

#[test]
fn scalar_field_names_and_saved_ir_retain_exact_contracts() {
    for (name, tag, _) in SCALARS {
        for import in ["using Toybox.Lang as L;", "import Toybox.Lang;"] {
            let ty = if import.starts_with("using") {
                format!("L.{name}")
            } else {
                name.to_string()
            };
            let source = format!(
                "{import} class C {{var value as {ty};var maybe as {ty} or Null=null;function initialize(v as {ty}){{self.value=v;}}function read() as {ty}{{return self.value;}}function write(v as {ty}) as Void{{self.value=v;}}function optional(v as {ty} or Null) as {ty} or Null{{self.maybe=v;var snapshot=self.maybe;if(snapshot!=null){{self.maybe=null;return snapshot;}}return null;}}}}"
            );
            let unit = compile_sources(&[&source]).unwrap();
            let text = unit.to_ir_text();
            assert_eq!(unit, parse_unit_ir(&text).unwrap());
            for edited in [
                text.replace(&format!(".field-scalar value {tag}\n"), ""),
                text.replace(
                    &format!(".field-scalar value {tag}"),
                    &format!(".field-scalar value nullable-{tag}"),
                ),
                text.replace(
                    &format!(".field-scalar maybe nullable-{tag}"),
                    &format!(".field-scalar maybe {tag}"),
                ),
                text.replace(
                    &format!(".field-scalar value {tag}"),
                    ".field-scalar value unknown",
                ),
                text.replace(
                    &format!(".field-scalar value {tag}"),
                    &format!(".field-scalar value nullable-nullable-{tag}"),
                ),
                text.replace(
                    &format!(".field-scalar value {tag}"),
                    ".field-scalar value numeric",
                ),
            ] {
                assert_ne!(text, edited);
                assert!(parse_unit_ir(&edited).is_err(), "{edited}");
            }
        }
    }
}

#[test]
fn scalar_fields_require_initialization_and_fresh_snapshot_proofs() {
    for (name, _, _) in SCALARS {
        for body in [
            "var value as Lang.TYPE;",
            "var value as Lang.TYPE;function initialize(){}",
            "var value as Lang.TYPE=null;",
            "var value as Lang.TYPE or Null=1;",
            "var value as Lang.TYPE;function initialize(v as Lang.TYPE or Null){self.value=v;}",
            "var value as Lang.TYPE;function initialize(v as Lang.TYPE){self.value=self.value;}",
            "var value as Lang.TYPE;function initialize(v as Lang.TYPE){if(true){self.value=v;}}",
            "var value as Lang.TYPE;function initialize(v as Lang.TYPE){System.exit();self.value=v;}",
            "var value as Lang.TYPE;function initialize(v as Lang.TYPE){return;self.value=v;}",
            "var value as Lang.TYPE;function initialize(v as Lang.TYPE){self.value=v;}function clear() as Void{self.value=null;}",
            "var value as Lang.TYPE or Null;function read() as Lang.TYPE{return self.value;}",
            "var value as Lang.TYPE or Null;function read() as Lang.TYPE{if(self.value!=null){return self.value;}return null;}",
        ] {
            let source = format!("class C {{{}}}", body.replace("TYPE", name));
            assert!(checking::compile_sources(&[&source]).is_err(), "{source}");
        }
        let source = format!(
            "class C {{var value as Lang.{name} or Null;function read() as Lang.{name} or Null{{var snapshot=self.value;if(snapshot!=null){{self.value=null;return snapshot;}}return null;}}}}"
        );
        let unit = compile_sources(&[&source]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
    for (name, bad) in [
        ("Number", "1l"),
        ("Long", "1"),
        ("Float", "1.0d"),
        ("Double", "1.0"),
        ("Boolean", "1"),
        ("Char", "65"),
        ("String", "1"),
    ] {
        let source = format!(
            "class C {{var value as Lang.{name};function initialize(){{self.value={bad};}}}}"
        );
        assert!(checking::compile_sources(&[&source]).is_err(), "{source}");
    }
}

#[test]
fn unused_scalar_fields_still_enforce_api_availability() {
    let low = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap();
    let char_api = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 3, 0)).unwrap();
    for (name, _, _) in SCALARS {
        let source =
            format!("class C {{var value as Lang.{name} or Null;function initialize(){{}}}}");
        let unit = compile_sources_with_api(&[&source], char_api).unwrap();
        assert_eq!(
            unit,
            parse_unit_ir_with_api(&unit.to_ir_text(), char_api).unwrap()
        );
        assert_eq!(
            compile_sources_with_api(&[&source], low).is_ok(),
            *name != "Char"
        );
        assert_eq!(
            parse_unit_ir_with_api(&unit.to_ir_text(), low).is_ok(),
            *name != "Char"
        );
    }
}
