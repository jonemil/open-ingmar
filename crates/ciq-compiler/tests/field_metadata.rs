use ciq_compiler::{FieldDefault, FieldKind, ReturnType, compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn literal_defaults_retain_tags_and_string_references_through_every_stage() {
    // Request runtime Symbol names so DATA fields can be identified independently
    // of the application's deterministic numeric-ID allocation.
    let mut source = include_str!("../../../examples/compiler-field-metadata/app.mc").to_owned();
    for name in [
        "flag",
        "ch",
        "flt",
        "lng",
        "dbl",
        "expr",
        "grouped",
        "groupedNumber",
        "groupedConst",
        "frozen",
    ] {
        source.push_str(&format!(
            "\nfunction id_{name}() as Toybox.Lang.Symbol {{ return :{name}; }}"
        ));
    }
    let unit = checking::compile_sources(&[&source]).unwrap();
    let prg = common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: [10; 16],
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    let data = prg.unique_section(ciq_prg::DATA).unwrap().payload();
    let symbols =
        ciq_prg::symbols::Symbols::parse(prg.unique_section(ciq_prg::SYMBOLS).unwrap().payload())
            .unwrap();
    let records = ciq_prg::data::classes::records(data).unwrap();
    let fields = records
        .iter()
        .filter_map(|record| match record {
            ciq_prg::data::classes::Record::Class(class) => Some(&class.fields),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    for (name, tag, value) in [
        ("flag", 9, 1),
        ("ch", 19, u32::from('Q')),
        ("flt", 2, 1.5f32.to_bits()),
        ("lng", 14, 0),
        ("dbl", 15, 0),
        ("expr", 0, 0),
        ("grouped", 0, 0),
        ("groupedNumber", 0, 0),
        ("groupedConst", 0, 0),
    ] {
        let id = symbols
            .iter()
            .find(|symbol| symbol.name == name)
            .unwrap()
            .id;
        let field = fields.iter().find(|field| field.symbol_id() == id).unwrap();
        assert_eq!((field.kind, field.value), (tag, value), "{name}");
    }
    let id = symbols
        .iter()
        .find(|symbol| symbol.name == "frozen")
        .unwrap()
        .id;
    let frozen = fields.iter().find(|field| field.symbol_id() == id).unwrap();
    assert_eq!(frozen.kind, 3);
    assert_eq!(frozen.key & 0xff, 0x10);
    assert_eq!(
        ciq_prg::data::string_at(data, frozen.value).unwrap().0,
        "fixed"
    );
}

#[test]
fn contracts_are_independent_of_literal_tags_and_quoted_text_round_trips() {
    let source = r#"using Toybox.Lang; class C {
        var text = "a # \"quote\"\nΩ";
        var flag = false;
        const frozen as Lang.String = "fixed";
        var wide as Lang.Double = 2.5d;
        var decimal as Lang.Decimal = -1.5f;
        function read() as Lang.String { return C.frozen; }
        function change() { text = 12; flag = "later"; }
    }"#;
    let unit = common::pipeline(compile_sources(&[source]).unwrap());
    let tabs = unit
        .to_ir_text()
        .replace(".field-metadata ", ".field-metadata\t");
    assert_eq!(unit, parse_unit_ir(&tabs).unwrap());
    assert_eq!(
        unit.classes()[0].fields[0].kind,
        FieldKind::Metadata {
            value_type: ReturnType::Any,
            initial: FieldDefault::String("a # \"quote\"\nΩ".into()),
            readonly: false,
        }
    );
    assert!(
        unit.to_ir_text()
            .contains(".field-metadata wide double mutable double 0")
    );
    assert!(
        unit.to_ir_text()
            .contains(".field-metadata decimal decimal mutable float 0xbfc00000")
    );
    assert!(compile_sources(&[&source.replace("text = 12", "frozen = \"changed\"")]).is_err());
}

#[test]
fn malformed_defaults_and_missing_wide_initialization_are_rejected() {
    let text = compile_sources(&[
        "class C { var text as Toybox.Lang.String=\"literal\"; var wide as Toybox.Lang.Long=1l; }",
    ])
    .unwrap()
    .to_ir_text();
    for bad in [
        text.replace("string mutable string", "integer mutable string"),
        text.replace("mutable string \"literal\"", "mutable boolean 2"),
        text.replace("mutable string \"literal\"", "mutable char 0xd800"),
        text.replace("mutable string \"literal\"", "mutable float 0x100000000"),
        text.replace("mutable string \"literal\"", "mutable symbol :bad.name"),
        text.replace("mutable string \"literal\"", "mutable string unquoted"),
        text.replace("mutable string \"literal\"", "mutable string \"unfinished"),
        text.replace("mutable string", "mutable, string"),
        text.replace("mutable long 0", "mutable long 1"),
        text.replace("mutable long 0", "mutable long"),
        text.lines()
            .filter(|line| !line.starts_with(".assign-self wide "))
            .collect::<Vec<_>>()
            .join("\n"),
    ] {
        assert_ne!(bad, text);
        assert!(parse_unit_ir(&bad).is_err(), "accepted malformed IR: {bad}");
    }
    for source in [
        "class C { var text as Toybox.Lang.Number=\"bad\"; }",
        "class C { var text as Toybox.Lang.String=true; }",
        "class C { const text as Toybox.Lang.String=1; }",
    ] {
        assert!(compile_sources(&[source]).is_err());
    }
}

#[test]
fn grouped_literals_keep_allocation_writes_and_class_definition_null_defaults() {
    let source = r#"class C {
        var grouped as Toybox.Lang.String=("text");
        var number as Toybox.Lang.Number=(7);
        const frozen as Toybox.Lang.String=("fixed");
        function read() as Toybox.Lang.String { return C.grouped; }
    }"#;
    let text = common::pipeline(checking::compile_sources(&[source]).unwrap()).to_ir_text();
    assert!(text.contains(".field-initialized grouped string"));
    assert!(text.contains(".field-initialized number integer"));
    assert!(text.contains(".field-constant frozen string"));
    assert!(
        text.lines()
            .any(|line| line.starts_with(".read-class ") && line.contains(" C grouped "))
    );
}
