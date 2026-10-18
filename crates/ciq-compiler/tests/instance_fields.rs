use ciq_compiler::{
    application::{ConsoleApplication, build_console_application},
    compile_sources, parse_ir, parse_unit_ir,
};

const APP: &str = include_str!("../../../examples/compiler-fields/app.mc");
const VIEW: &str = include_str!("../../../examples/compiler-fields/view.mc");

#[test]
fn field_state_preserves_complete_pipelines_and_instance_metadata() {
    let unit = compile_sources(&[VIEW, APP]).unwrap();
    assert_eq!(unit.classes()[0].fields[0].name, "count");
    assert_eq!(
        unit.classes()[0].fields[0].kind,
        ciq_compiler::FieldKind::Number(7)
    );
    let text = unit.to_ir_text();
    let resumed = parse_unit_ir(&text).unwrap();
    assert_eq!(unit, resumed);
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    let options = ConsoleApplication {
        id: [50; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    let prg = build_console_application(&unit, &options).unwrap();
    assert_eq!(prg, build_console_application(&resumed, &options).unwrap());
    let records =
        ciq_prg::data::classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
            .unwrap();
    let view = records
        .iter()
        .find_map(|record| match record {
            ciq_prg::data::classes::Record::Class(class) if class.extends == Some(0x40000001) => {
                Some(class)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(
        view.fields
            .iter()
            .filter(|field| field.kind == 1 && field.value == 7 && field.key & 0xff == 0)
            .count(),
        1
    );
    assert!(
        view.fields
            .windows(2)
            .all(|fields| fields[0].key < fields[1].key)
    );
    let code = ciq_mc::decode(prg.unique_section(ciq_prg::CODE).unwrap().payload()).unwrap();
    assert!(
        code.iter()
            .any(|op| op.instruction.opcode() == ciq_mc::Opcode::Putv)
    );
}

#[test]
fn declarations_and_saved_ir_resolve_fields_against_their_owner() {
    for body in [
        "var n = 1; var n = 2; function start() {}",
        "var n = 1; function n() {}",
        "var initialize = 1; function start() {}",
        "var AppBase = 1; function start() {}",
        "var n; function start() {}",
        "var n = null; function start() {}",
        "var n = 1 + 2; function start() {}",
        "var n = 1; function start() { self.missing = 2; }",
        "var n = 1; function start() { System.println(self.missing); }",
        "var n = 1; function start() { self.n = \"wrong\"; }",
    ] {
        let source = format!("class C extends Application.AppBase {{ {body} }}");
        assert!(compile_sources(&[&source]).is_err(), "{source}");
    }
    let unit = compile_sources(&[APP, VIEW]).unwrap();
    let text = unit.to_ir_text();
    for changed in [
        text.replace(".field-number count 7\n", ""),
        text.replace(
            ".field-number count 7",
            ".field-number count 7\n.field-number count 8",
        ),
        text.replace(".field-number count 7", ".field-number other 7"),
        text.replace(".assign-self count", ".assign-self missing"),
    ] {
        assert_ne!(text, changed);
        assert!(parse_unit_ir(&changed).is_err(), "{changed}");
    }
    assert!(parse_ir("ciq-ir\n.function f\n.read-self %0 n\n.return %0\n.end\n").is_err());
}

#[test]
fn field_reads_cannot_move_across_writes_and_locals_remain_distinct() {
    let source = "class C extends Application.AppBase { var n = -2147483648; function start() { var n = self.n; self.n = 2; System.println(n); System.println(self.n); } }";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(
        unit.classes()[0].fields[0].kind,
        ciq_compiler::FieldKind::Number(i32::MIN)
    );
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let bad = "ciq-ir\n.function f receiver=instance\n.read-self %0 n\n.integer %1 2\n.assign-self n %1\n.println %0\n.return-void\n.end\n";
    assert!(
        parse_ir(bad)
            .unwrap_err()
            .message
            .contains("evaluation order")
    );
    let write = "ciq-ir\n.function f receiver=instance\n.string %0 \"bad\"\n.assign-self n %0\n.return-void\n.end\n";
    assert!(parse_ir(write).is_ok()); // An interface field may hold String.
    assert!(
        ciq_compiler::parse_unit_ir(&format!(
            "ciq-unit\n.class C Lang.Object\n.field-number n 0\n{write}.end-class\n"
        ))
        .is_err()
    );
}
