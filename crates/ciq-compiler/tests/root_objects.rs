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
            "{declaration} {{ var n=1; function initialize(n as Toybox.Lang.Number) {{ self.n=n; }} function add(n as Toybox.Lang.Number) as Toybox.Lang.Number {{ self.n=self.n+n; return self.n; }} }} function f() as Toybox.Lang.Number {{ var c=new C(3); return c.add(4); }}"
        );
        let unit = compile_sources_with_api(&[&source], api).unwrap();
        assert_eq!(unit.classes()[0].base, ClassBase::LangObject);
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
    let source = "class C { function initialize() {} } module Factory { function make() as Toybox.Lang.Object { return new C(); } } function f() as Toybox.Lang.String { return Factory.make().toString(); }";
    assert!(compile_sources_with_api(&[source], api).is_ok());
    let source = "class C { function initialize() {} function hashCode() as Toybox.Lang.Number { return 17; } function value() as Toybox.Lang.Number { return self.hashCode(); } } function f() as Toybox.Lang.Number { var c=new C(); return c.hashCode()+c.value(); }";
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
        assert_eq!(roots, 5);
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
        "class C { function Object() as Void {} function initialize() {} }",
        "class C { function initialize() { View.initialize(); } }",
        "class C { function initialize() { Object.initialize(); } }",
        "class C { function initialize() { return 1; } }",
        "class C { function initialize(n as Toybox.Lang.Number) {} } function f() as Void { var c=new C(); }",
        "class C { function initialize() {} } class Holder { var value as C; function initialize() {} }",
        "class C { function initialize() {} } function f() as Toybox.WatchUi.View { return new C(); }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { return o.value(); }",
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

#[test]
fn exact_references_resolve_across_files_without_erasing_native_bases() {
    let classes = "class C { function initialize() {} function value() as Toybox.Lang.Number { return 7; } } class Holder { var target as C; function initialize(c as C) { self.target=c; } function get() as C { return self.target; } }";
    let functions = "module M { function pass(c as C) as C { return c; } function make() as C { return new C(); } } function f() as Toybox.Lang.Number { var h=new Holder(M.pass(M.make())); return h.get().value(); }";
    for sources in [[functions, classes], [classes, functions]] {
        let unit = compile_sources(&sources).unwrap();
        let text = unit.to_ir_text();
        assert!(text.contains("parameter-types=source(C@Lang.Object)"));
        assert!(text.contains("result=source(C@Lang.Object)"));
        assert!(text.contains(".field-reference target source(C@Lang.Object)"));
        assert_eq!(unit, parse_unit_ir(&text).unwrap());
        for changed in [
            text.replace("C@Lang.Object", "C@WatchUi.View"),
            text.replace("source(C@Lang.Object)", "source(Missing@Lang.Object)"),
            text.replace(
                ".field-reference target source(C@Lang.Object)",
                ".field-reference target source(C@WatchUi.View)",
            ),
            text.replace(
                "M make result=source(C@Lang.Object)",
                "M make result=object",
            ),
        ] {
            assert_ne!(text, changed);
            assert!(parse_unit_ir(&changed).is_err(), "{changed}");
        }
    }
    // Exact delegate references use the same model, without projecting to View.
    let unit = compile_sources(&["class D extends Toybox.WatchUi.BehaviorDelegate { function initialize() { BehaviorDelegate.initialize(); } } module M { function pass(d as D) as D { return d; } } function f() as Toybox.WatchUi.BehaviorDelegate { return M.pass(new D()); }"]).unwrap();
    assert!(
        unit.to_ir_text()
            .contains("source(D@WatchUi.BehaviorDelegate)")
    );
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn root_reference_contracts_reject_erasure_wrong_classes_and_early_exposure() {
    let declarations = "class C { function initialize() {} } class Other { function initialize() {} } module M { function pass(c as C) as C { return c; } }";
    for body in [
        "function f(o as Toybox.Lang.Object) as Void { M.pass(o); }",
        "function f() as Void { M.pass(new Other()); }",
        "function f() as Void { M.pass(null); }",
        "function f(c as Other) as C { return c; }",
        "function f(o as Toybox.Lang.Object) as C { return o; }",
        "function f() as C { return new Other(); }",
        "function f(c as Missing) as Toybox.Lang.Number { return 0; }",
        "class H { var c as C; function initialize() {} }",
        "class H { var c as C; function initialize(c as C) { self.c = new Other(); } }",
        "class H { var c as C; function initialize(c as C) { var x=self.c; self.c=c; } }",
        "class H { var c as C; function initialize(c as C) { var x=self; self.c=c; } }",
        "class H { var c as C; function initialize(c as C) { System.exit(); self.c=c; } }",
    ] {
        assert!(compile_sources(&[body, declarations]).is_err(), "{body}");
    }
    for tag in [
        "source(C)",
        "source(@Lang.Object)",
        "source(C@Application.AppBase)",
        "source(C@Unknown)",
        "source(C@Lang.Object@WatchUi.View)",
    ] {
        let text = format!(
            "ciq-ir\n.function f parameters=1 parameter-types={tag} result={tag}\n.parameter %0 0\n.return %0\n.end\n"
        );
        assert!(parse_ir(&text).is_err(), "{tag}");
    }
}

#[test]
fn single_function_source_cannot_guess_an_unresolved_class_base() {
    let source = "function f(c as C) as C { return c; }";
    let error = ciq_compiler::compile(source).unwrap_err();
    assert!(error.to_string().contains("compilation-unit declarations"));
}

#[test]
fn default_object_initializers_are_explicit_in_ir_and_keep_field_requirements() {
    for declaration in [
        "class C {}",
        "class C extends Toybox.Lang.Object {}",
        "class C { var n=7; var text as Toybox.Lang.String or Null; function value() as Toybox.Lang.Number {return self.n;} }",
    ] {
        let source = format!("{declaration} function f() as C {{return new C();}}");
        let unit = compile_sources(&[&source]).unwrap();
        let initializer = unit.classes()[0]
            .methods
            .iter()
            .find(|m| m.name() == "initialize")
            .unwrap();
        assert_eq!(initializer.parameter_count(), 0);
        assert_eq!(initializer.return_type(), ciq_compiler::ReturnType::Void);
        assert!(
            initializer
                .operations()
                .iter()
                .all(|op| matches!(op.operation, ciq_compiler::Operation::ReturnVoid))
        );
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
        let assembly = unit.lower();
        assert_eq!(
            assembly,
            ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
        );
    }
    // A source initializer replaces the default, rather than adding an overload.
    let unit = compile_sources(&[
        "class C {function initialize(n as Toybox.Lang.Number) {}} function f() as C {return new C(1);}",
    ])
    .unwrap();
    assert_eq!(unit.classes()[0].methods.len(), 1);
    assert_eq!(unit.classes()[0].methods[0].parameter_count(), 1);
    for source in [
        "class C {} function f() as C {return new C(1);}",
        "class C {var text as Toybox.Lang.String;}",
        "class C {var target as C;}",
        "class C {var initialize=1;}",
        "class C extends Toybox.WatchUi.View {function value() as Toybox.Lang.Number {return 1;}} function f() as Toybox.WatchUi.View {return new C();}",
        "module Empty {}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "accepted {source}");
    }
    // Saved IR must retain the normalized body; its reader does not silently
    // repair missing method declarations or bypass constructor checking.
    let unit = compile_sources(&["class C {} function f() as C {return new C();}"]).unwrap();
    let text = unit.to_ir_text();
    let start = text.find(".function initialize").unwrap();
    let end = start + text[start..].find(".end\n").unwrap() + ".end\n".len();
    let mut missing = text;
    missing.replace_range(start..end, "");
    assert!(parse_unit_ir(&missing).is_err());
}

#[test]
fn default_initializers_count_toward_unit_body_limit() {
    let source = (0..256)
        .map(|i| format!("class C{i} {{}}\n"))
        .collect::<String>();
    let unit = compile_sources(&[&source]).unwrap();
    assert_eq!(unit.classes().len(), 256);
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let too_many = format!("{source} class Extra {{}}");
    let error = compile_sources(&[&too_many]).unwrap_err();
    assert!(error.diagnostic.message.contains("256-function limit"));
}
