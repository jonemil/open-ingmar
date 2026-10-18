use ciq_compiler::{
    FieldKind,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_sources, compile_sources_with_api, parse_unit_ir, parse_unit_ir_with_api,
};

const SOURCE: &str = "using Toybox.Lang; class Box {
    var fixed as Lang.ByteArray;
    var maybe as Lang.ByteArray or Null = null;
    function initialize(b as Lang.ByteArray) {self.fixed=b;}
    function get() as Lang.ByteArray {return self.fixed;}
    function optional() as Lang.ByteArray or Null {return self.maybe;}
    function put(b as Lang.ByteArray or Null) as Void {self.maybe=b;}
    function change() as Void {self.fixed[0]+=1;var b=self.maybe;if(b!=null){b[0]='A';}}
}";

#[test]
fn byte_array_fields_preserve_storage_metadata_and_saved_ir_contracts() {
    let unit = compile_sources(&[SOURCE]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".field-byte-array fixed byte-array\n"));
    assert!(text.contains(".field-byte-array maybe nullable-byte-array\n"));
    assert_eq!(
        unit.classes()[0].fields[0].kind,
        FieldKind::ByteArray { nullable: false }
    );
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    let asm = unit.lower();
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    let options = ConsoleApplication {
        id: [0x72; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    let app = compile_sources(&[SOURCE, "class MinimalConsoleApp extends Toybox.Application.AppBase {function initialize(){AppBase.initialize();}function onStart(state){var box=new Box([7]b);box.change();}function getInitialView(){return [new Toybox.WatchUi.View()];}} "]).unwrap();
    let prg = build_console_application(&app, &options).unwrap();
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
    assert_eq!(slots.len(), 2);
    assert!(slots.iter().all(|field| field.value == 0));
    for edited in [
        text.replace(
            ".field-byte-array fixed byte-array",
            ".field-byte-array fixed nullable-byte-array",
        ),
        text.replace(
            ".field-byte-array maybe nullable-byte-array",
            ".field-byte-array maybe byte-array",
        ),
        text.replace(
            ".field-byte-array maybe nullable-byte-array",
            ".field-byte-array maybe nullable-method",
        ),
        text.replace(
            "maybe result=nullable-byte-array",
            "maybe result=byte-array",
        ),
    ] {
        assert_ne!(text, edited);
        assert!(parse_unit_ir(&edited).is_err(), "accepted {edited}");
    }
}

#[test]
fn byte_array_fields_require_initialization_and_snapshot_null_proofs() {
    for source in [
        "class C {var b as Toybox.Lang.ByteArray;}",
        "class C {var b as Toybox.Lang.ByteArray;function initialize(){}}",
        "class C {var b as Toybox.Lang.ByteArray=null;}",
        "class C {var b as Toybox.Lang.ByteArray=[1]b;}",
        "class C {var b as Toybox.Lang.ByteArray or Null=[1]b;}",
        "class C {var b as Toybox.Lang.ByteArray;function initialize(b as Toybox.Lang.ByteArray or Null){self.b=b;}}",
        "class C {var b as Toybox.Lang.ByteArray;function initialize(){self.b=[1];}}",
        "class C {var b as Toybox.Lang.ByteArray;function initialize(){self.b=self.b;}}",
        "class C {var b as Toybox.Lang.ByteArray;function initialize(){if(true){self.b=[1]b;}}}",
        "class C {var b as Toybox.Lang.ByteArray;function initialize(){self.f();self.b=[1]b;}function f() as Void {}}",
        "class C {var b as Toybox.Lang.ByteArray or Null;function f() {return self.b.size();}}",
        "class C {var b as Toybox.Lang.ByteArray or Null;function f() {if(self.b!=null){return self.b[0];}return 0;}}",
        "class C {var b as Toybox.Lang.ByteArray or Null;function f() as Void {self.b=[1];}}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "accepted {source}");
    }
    let source = "class C {var b as Toybox.Lang.ByteArray or Null;function f() {var snapshot=self.b;if(snapshot!=null){self.b=null;return snapshot[0];}return 0;}}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn unused_nullable_byte_array_fields_keep_their_api_requirement() {
    let source = "class C {var b as Toybox.Lang.ByteArray or Null;function f() as Void {}}";
    let modern = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(3, 0, 0)).unwrap();
    let old = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(2, 4, 0)).unwrap();
    let unit = compile_sources_with_api(&[source], modern).unwrap();
    assert!(compile_sources_with_api(&[source], old).is_err());
    assert!(parse_unit_ir_with_api(&unit.to_ir_text(), old).is_err());
}
