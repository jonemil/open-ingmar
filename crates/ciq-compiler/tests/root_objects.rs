use ciq_compiler::{
    ClassBase,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_sources, compile_sources_with_api, parse_ir, parse_unit_ir,
};

const APP: &str = include_str!("../../../examples/compiler-root-objects/app.mc");
const OBJECTS: &str = include_str!("../../../examples/compiler-root-objects/objects.mc");

#[test]
fn implicit_and_resolved_object_bases_work_at_the_initial_api_level() {
    let api = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    for declaration in [
        "class C",
        "class C extends Toybox.Lang.Object",
        "class C extends Lang.Object",
        "using Toybox.Lang as L; class C extends L.Object",
        "import Toybox.Lang; class C extends Object",
    ] {
        let source = format!(
            "{declaration} {{ var n=1; function initialize(n) {{ self.n=n; }} function add(n) {{ self.n=self.n+n; return self.n; }} }} function f() {{ var c=new C(3); return c.add(4); }}"
        );
        let unit = compile_sources_with_api(&[&source], api).unwrap();
        assert_eq!(unit.classes()[0].base, ClassBase::LangObject);
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
    let source = "class C { function initialize() {} } module Factory { function make() as Toybox.Lang.Object { return new C(); } } function f() as Toybox.Lang.String { return Factory.make().toString(); }";
    assert!(compile_sources_with_api(&[source], api).is_ok());
    let source = "class C { function initialize() {} function hashCode() { return 17; } function value() { return self.hashCode(); } } function f() { var c=new C(); return c.hashCode()+c.value(); }";
    let unit = compile_sources(&[source]).unwrap();
    assert!(unit.to_ir_text().contains(".call-self"));
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn native_object_import_and_hidden_constructor_survive_complete_saved_paths() {
    for delegate in [
        "",
        "class D extends Toybox.WatchUi.BehaviorDelegate { function initialize() { BehaviorDelegate.initialize(); } }",
    ] {
        let unit = compile_sources(&[APP, OBJECTS, delegate]).unwrap();
        let text = unit.to_ir_text();
        assert!(text.contains(".class Counter Lang.Object"));
        assert!(text.contains("Counter Lang.Object"));
        let resumed = parse_unit_ir(&text).unwrap();
        assert_eq!(unit, resumed);
        let options = ConsoleApplication {
            id: [0x98; 16],
            entry: "MinimalConsoleApp.onStart".into(),
        };
        let prg = build_console_application(&unit, &options).unwrap();
        assert_eq!(prg, build_console_application(&resumed, &options).unwrap());
        let imports = prg.unique_section(0xc1a557b1).unwrap().payload();
        let index = if delegate.is_empty() { 2 } else { 3 };
        assert_eq!(
            u16::from_be_bytes(imports[..2].try_into().unwrap()) as usize,
            index + 1
        );
        assert_eq!(
            &imports[2 + index * 8..2 + (index + 1) * 8],
            &[0x00, 0x80, 0x00, 0x07, 0x00, 0x80, 0x00, 0x4c]
        );
        let records =
            ciq_prg::data::classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
                .unwrap();
        let code = prg.unique_section(ciq_prg::CODE).unwrap().payload();
        let mut roots = 0;
        for record in records {
            if let ciq_prg::data::classes::Record::Class(class) = record {
                if class.extends == Some(0x40000000 | index as u32) {
                    roots += 1;
                    let ctor = class
                        .fields
                        .iter()
                        .find(|field| field.key >> 8 == 0x00800018)
                        .unwrap();
                    assert_eq!(
                        code[(ctor.value - 0x10000000) as usize],
                        ciq_mc::Opcode::Return as u8
                    );
                    assert!(class.fields.iter().any(|field| field.key >> 8 == 0x0080004c
                        && field.value == 0x40000000 | index as u32));
                }
            }
        }
        assert_eq!(roots, 2);
        let asm = unit.lower();
        assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    }
}

#[test]
fn constructor_identity_scope_and_view_specific_contracts_stay_checked() {
    for source in [
        "using Toybox.Lang; class C extends Object { function initialize() {} }",
        "using Toybox.Lang as L; class L { function initialize() {} }",
        "import Toybox.Lang; class Object { function initialize() {} }",
        "class Object { function initialize() {} }",
        "class C { var Object=1; function initialize() {} }",
        "class C { function Object() {} function initialize() {} }",
        "class C { function initialize() { View.initialize(); } }",
        "class C { function initialize() { Object.initialize(); } }",
        "class C { function initialize() { return 1; } }",
        "class C { function value() { return 1; } } function f() { var c=new C(); }",
        "class C { function initialize(n) {} } function f() { var c=new C(); }",
        "class C { function initialize() {} } function f(c as C) { return 1; }",
        "class C { function initialize() {} } function f() as C { return new C(); }",
        "class C { function initialize() {} } class Holder { var value as C; function initialize() {} }",
        "class C { function initialize() {} } function f() { var c=new C(); var d=new Other(); c=d; } class Other { function initialize() {} }",
        "class C { function initialize() {} } function f() as Toybox.WatchUi.View { return new C(); }",
        "class C { function initialize() {} } function f() { var a=[new C()]; }",
        "function f(o as Toybox.Lang.Object) { return o.value(); }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "accepted {source}");
    }
    assert!(parse_ir("ciq-ir\n.function initialize receiver=instance\n.base-initialize Lang.Object\n.return-void\n.end\n").is_err());
    let unit = compile_sources(&[APP, OBJECTS]).unwrap();
    let text = unit.to_ir_text();
    let collision = text.replace(".field-number count 1", ".field-number Object 1");
    assert!(collision != text);
    assert!(parse_unit_ir(&collision).is_err());
    let bad = text.replace(
        ".new-source %1 Counter Lang.Object",
        ".new-source %1 Counter WatchUi.View",
    );
    assert!(bad != text);
    assert!(parse_unit_ir(&bad).is_err());
}
