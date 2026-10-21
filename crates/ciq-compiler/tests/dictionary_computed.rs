use ciq_compiler::{Operation, compile, compile_sources, parse_ir, parse_unit_ir};
use ciq_mc::Opcode;

#[test]
fn computed_entries_use_ordered_private_construction_and_native_text_paths() {
    let source = "module D {
        function key(n) as Toybox.Lang.String { return n == 1 ? \"one\" : \"two\"; }
        function value(n) { return n + 1; }
        function make() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number> {
            return {D.key(1) => D.value(1), D.key(2) => D.value(2)};
        }
    }";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let text = unit.to_ir_text();
    assert!(text.contains(".dictionary-begin"));
    assert!(text.contains(".dictionary-fill"));
    assert!(text.contains(".dictionary-finish"));
    assert!(!text.contains(".dictionary-constant"));
    let p = unit.lower();
    assert_eq!(p, ciq_assembly::Program::parse(&p.to_text()).unwrap());
    assert!(p.items().iter().any(|item| matches!(
        item,
        ciq_assembly::Item::Instruction {
            opcode: Opcode::Aputvdup,
            ..
        }
    )));
    let begin = text.find(".dictionary-begin").unwrap();
    assert!(begin < text[begin..].find(" D key ").unwrap() + begin);
}

#[test]
fn conditional_operands_and_scalar_array_producers_preserve_builder_progress() {
    for body in [
        "return {choose ? \"one\" : \"two\" => choose ? n : n + 1, \"three\" => n + 2};",
        "return {\"one\" => choose ? n : null, \"two\" => n + 1};",
        "return {\"one\" => [n, n + 1][0], \"two\" => n + 2};",
    ] {
        let source = format!(
            "function f(choose as Toybox.Lang.Boolean, n as Toybox.Lang.Number) as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number or Null> {{ {body} }}"
        );
        let f = compile(&source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        assert_eq!(
            f.lower(),
            ciq_assembly::Program::parse(&f.lower().to_text()).unwrap()
        );
    }
    let source = "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { if (n != null) { var d = {n => n + 1}; return d.size(); } return 0; }";
    let f = compile(source).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    assert!(compile(&source.replace("if (n != null)", "if (true)")).is_err());
    let source = "function f(k as Toybox.Lang.String) as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number or Null> { return {k => null}; }";
    let f = compile(source).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let inferred = compile("function f(k as Toybox.Lang.String, n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { var d = {k => n}; return d.size(); }").unwrap();
    assert!(inferred.operations().iter().any(|op| matches!(op.operation, Operation::DictionaryBegin { dictionary_type, .. } if dictionary_type.value == ciq_compiler::DictionaryValue::NullableNumber)));
}

#[test]
fn builders_cannot_escape_early_or_skip_reorder_or_repeat_source_entries() {
    let good = "ciq-ir\n.function f\n.dictionary-begin &0 dictionary(integer=>integer) 2\n.integer %0 1\n.integer %1 2\n.integer %2 1\n.integer %3 3\n.dictionary-fill &0 0 %0 %1 %2 %3\n.dictionary-finish %4 &0\n.call-native %5 %4 size\n.return %5\n.end\n";
    let f = parse_ir(good).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    for bad in [
        good.replace(" 2\n.integer", " 0\n.integer"),
        good.replace(" 2\n.integer", " 2147483648\n.integer"),
        good.replace("&0 0 %0 %1 %2 %3", "&0 1 %0 %1 %2 %3"),
        good.replace("&0 0 %0 %1 %2 %3", "&0 0 %0 %1"),
        good.replace(".dictionary-finish %4 &0", ".dictionary-finish %4 &99"),
        good.replace(".dictionary-finish %4 &0", ".array-finish %4 &0"),
        good.replace(
            ".dictionary-fill &0 0 %0 %1 %2 %3",
            ".array-fill-many &0 0 %0 %1 %2 %3",
        ),
        good.replace(
            ".dictionary-finish %4 &0",
            ".dictionary-finish %4 &0\n.dictionary-finish %6 &0",
        ),
        good.replace(
            ".dictionary-begin &0",
            ".dictionary-begin &0 dictionary(integer=>integer) 2\n.dictionary-begin &0",
        ),
        good.replace(
            ".dictionary-begin &0 dictionary(integer=>integer) 2",
            ".array-begin &0 number 2",
        ),
        good.replace(
            ".dictionary-fill &0 0 %0 %1 %2 %3",
            ".dictionary-fill &0 0 %0",
        ),
    ] {
        assert!(parse_ir(&bad).is_err(), "accepted {bad}");
    }
    let unfinished = "ciq-ir\n.function f\n.dictionary-begin &0 dictionary(integer=>integer) 1\n.integer %0 0\n.return %0\n.end\n";
    assert!(
        parse_ir(unfinished)
            .unwrap_err()
            .message
            .contains("unfinished")
    );
}

#[test]
fn computed_types_do_not_accept_mixed_contracts_or_nested_dictionary_literals() {
    for source in [
        "function f(k as Toybox.Lang.String) as Toybox.Lang.Number { var d = {k => null}; return d.size(); }",
        "function f(n as Toybox.Lang.Number) as Toybox.Lang.Number { var d = {n => 1, \"x\" => n}; return d.size(); }",
        "function f(n as Toybox.Lang.Number) as Toybox.Lang.Number { var d = {\"x\" => n, \"y\" => \"bad\"}; return d.size(); }",
        "function f(n as Toybox.Lang.Number) as Toybox.Lang.Number { var d = {\"x\" => [n]}; return d.size(); }",
        "function f(n as Toybox.Lang.Number) as Toybox.Lang.Number { var d = {\"x\" => ({\"inner\" => n})}; return d.size(); }",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
}

#[test]
fn completion_progress_must_match_on_both_incoming_edges() {
    let text = "ciq-ir\n.function f parameters=1 parameter-types=boolean\n.block ^0\n.dictionary-begin &0 dictionary(integer=>integer) 1\n.parameter %0 0\n.branch %0 ^1 ^2\n.block ^1\n.integer %1 1\n.integer %2 2\n.dictionary-fill &0 0 %1 %2\n.jump ^3\n.block ^2\n.jump ^3\n.block ^3\n.dictionary-finish %3 &0\n.call-native %4 %3 size\n.return %4\n.end\n";
    assert!(
        parse_ir(text)
            .unwrap_err()
            .message
            .contains("progress must agree")
    );
}

#[test]
fn dictionary_builders_and_staging_obey_the_existing_frame_limit() {
    let parameters = (0..254)
        .map(|n| format!("p{n}"))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        "function f({parameters}) as Toybox.Lang.Dictionary<Toybox.Lang.Number, Toybox.Lang.Number> {{ return {{p0 => p1}}; }}"
    );
    let f = compile(&source).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let code = f.lower().encode().unwrap();
    assert_eq!(&code[..3], &[0x4a, 255, 1]);
    let staged = source.replace("p0 => p1", "(true ? p0 : p1) => p1");
    assert!(compile(&staged).unwrap_err().message.contains("255"));
}
