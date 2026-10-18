use ciq_compiler::{
    FieldKind, Operation,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_sources, compile_sources_with_api, parse_unit_ir,
};

const APP: &str = include_str!("../../../examples/compiler-symbols/app.mc");

#[test]
fn symbol_fields_preserve_saved_stages_and_metadata_defaults() {
    let unit = compile_sources(&[APP]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".field-symbol required symbol\n"));
    assert!(text.contains(".field-symbol preset symbol default=:metadataOnly\n"));
    assert!(
        text.contains(".field-symbol optionalPreset nullable-symbol default=:optionalMetadata\n")
    );
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    let store = unit
        .classes()
        .iter()
        .find(|class| class.name == "SymbolStore")
        .unwrap();
    assert_eq!(
        store.fields[0].kind,
        FieldKind::Symbol {
            initial: None,
            nullable: false
        }
    );
    assert_eq!(
        store.fields[1].kind,
        FieldKind::Symbol {
            initial: None,
            nullable: true
        }
    );
    let options = ConsoleApplication {
        id: [0x51; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    let prg = build_console_application(&unit, &options).unwrap();
    assert_eq!(
        prg,
        build_console_application(&parse_unit_ir(&text).unwrap(), &options).unwrap()
    );
    let symbols =
        ciq_prg::symbols::Symbols::parse(prg.unique_section(ciq_prg::SYMBOLS).unwrap().payload())
            .unwrap();
    let classes =
        ciq_prg::data::classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
            .unwrap();
    let defaults = classes
        .iter()
        .flat_map(|record| match record {
            ciq_prg::data::classes::Record::Class(class) => class.fields.as_slice(),
            _ => &[],
        })
        .filter(|field| field.kind == 8)
        .map(|field| field.value)
        .collect::<Vec<_>>();
    for name in [
        "appMetadataOnly",
        "metadataOnly",
        "optionalMetadata",
        "size",
    ] {
        let symbol = symbols.iter().find(|symbol| symbol.name == name).unwrap();
        assert!(defaults.contains(&symbol.id), "{name}");
    }
    assert_eq!(
        symbols
            .iter()
            .find(|symbol| symbol.name == "size")
            .unwrap()
            .id,
        0x008000ae
    );
    for edited in [
        text.replace(
            ".field-symbol preset symbol default=:metadataOnly",
            ".field-symbol preset symbol default=0x1",
        ),
        text.replace(
            ".field-symbol preset symbol default=:metadataOnly",
            ".field-symbol preset string default=:metadataOnly",
        ),
        text.replace(
            ".field-symbol maybe nullable-symbol",
            ".field-symbol maybe symbol",
        ),
        text.replace(
            ".field-symbol preset symbol default=:metadataOnly",
            ".field-symbol preset nullable-symbol default=:metadataOnly",
        ),
    ] {
        assert_ne!(text, edited);
        assert!(parse_unit_ir(&edited).is_err());
    }
}

#[test]
fn metadata_only_symbol_names_are_allocated_without_spush_operands() {
    let source = "using Toybox.Application; class App extends Application.AppBase {var root as Toybox.Lang.Symbol=:rootOnly; function initialize(){AppBase.initialize();} function onStart(state){}} class C {var value=:classOnly; function initialize(){}}";
    let unit = compile_sources(&[source]).unwrap();
    assert!(
        !unit
            .classes()
            .iter()
            .flat_map(|c| &c.methods)
            .flat_map(|m| m.operations())
            .any(|op| matches!(op.operation, Operation::Symbol { .. }))
    );
    let prg = build_console_application(
        &unit,
        &ConsoleApplication {
            id: [0x52; 16],
            entry: "App.onStart".into(),
        },
    )
    .unwrap();
    let symbols =
        ciq_prg::symbols::Symbols::parse(prg.unique_section(ciq_prg::SYMBOLS).unwrap().payload())
            .unwrap();
    for name in ["rootOnly", "classOnly"] {
        assert!(symbols.iter().any(|symbol| symbol.name == name));
    }
    let initial = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap();
    assert!(compile_sources_with_api(&[source], initial).is_ok());
}

#[test]
fn required_symbols_must_be_assigned_before_reads_calls_or_return() {
    for body in [
        "",
        "var s=self.value;self.value=:alpha;",
        "self.call();self.value=:alpha;",
        "var s=self;self.value=:alpha;",
        "if(true){self.value=:alpha;}else{self.value=:beta;}",
        "self.value=null;",
        "self.value=1;",
        "Toybox.System.exit();self.value=:alpha;",
    ] {
        let source = format!(
            "class C {{var value as Toybox.Lang.Symbol;function initialize(){{{body}}}function call(){{return 0;}}}}"
        );
        assert!(compile_sources(&[&source]).is_err(), "{source}");
    }
    for source in [
        "class C {var value as Toybox.Lang.Symbol;function other(){return 0;}}",
        "class C {var value as Toybox.Lang.Symbol=null;function initialize(){}}",
        "class C {var value as Toybox.Lang.Symbol=1;function initialize(){}}",
        "class C {var value as Toybox.Lang.Symbol=:alpha;function initialize(){self.value=null;}}",
        "class C {var value as Toybox.Lang.Symbol or Null;function initialize(){self.value=1;}}",
        "class C {var value=:alpha;function initialize(){self.value=null;}}",
        "class C {var value as Toybox.Lang.Symbol=:alpha.toString();function initialize(){}}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let valid = "class C {var value as Toybox.Lang.Symbol;function initialize(s as Toybox.Lang.Symbol){self.value=s;}function read() as Toybox.Lang.Symbol{return self.value;}}";
    let unit = compile_sources(&[valid]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn nullable_symbol_fields_need_local_snapshots_and_rechecked_ir_types() {
    let source = "class C {var value as Toybox.Lang.Symbol or Null;function initialize(){}function read() as Toybox.Lang.String {var snapshot=self.value;self.value=null;if(snapshot!=null){return snapshot.toString();}return \"missing\";}}";
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    let edited = text.replace("value result=nullable-symbol", "value result=symbol");
    assert_ne!(text, edited);
    assert!(parse_unit_ir(&edited).is_err());
    let repeated = "class C {var value as Toybox.Lang.Symbol or Null;function initialize(){}function read() as Toybox.Lang.String {if(self.value!=null){return self.value.toString();}return \"missing\";}}";
    assert!(compile_sources(&[repeated]).is_err());
}
