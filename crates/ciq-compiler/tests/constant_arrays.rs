use ciq_assembly::{Item, Program};
use ciq_compiler::{ArrayElement, Operation, array_bindings, compile, parse_ir};
use ciq_prg::data::Source;
use std::collections::BTreeMap;

#[test]
fn scalar_literals_remain_typed_through_ir_assembly_and_explicit_data_binding() {
    for (elements, ty) in [
        ("0, 2147483647, -2147483648", "number"),
        ("'a', 'Ω'", "char"),
    ] {
        let f = compile(&format!(
            "function f() as Toybox.Lang.Array<Toybox.Lang.{}> {{ return [{elements}]; }}",
            if ty == "number" { "Number" } else { "Char" }
        ))
        .unwrap();
        assert!(f.to_ir_text().contains(&format!(".array-constant %0 {ty}")));
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        let assembly = f.lower();
        assert_eq!(assembly, Program::parse(&assembly.to_text()).unwrap());
        assert!(
            assembly
                .encode()
                .unwrap_err()
                .message
                .contains("undefined array alias")
        );
        let records = assembly
            .items()
            .iter()
            .filter_map(|item| match item {
                Item::Array(a) => Some(a.to_text().replace(&a.alias, "placed")),
                _ => None,
            })
            .collect::<String>();
        let data = Source::parse(&format!(".string prefix \"before\"\n{records}")).unwrap();
        let arrays = array_bindings(&assembly, &data).unwrap();
        let code = assembly
            .encode_with_literals(&BTreeMap::new(), &BTreeMap::new(), &arrays)
            .unwrap();
        let expected_offset = data.array_bindings().unwrap()["placed"];
        let loads = ciq_mc::decode(&code)
            .unwrap()
            .into_iter()
            .filter(|i| i.instruction.opcode() == ciq_mc::Opcode::Apush)
            .count();
        assert_eq!(loads, 1);
        let operand = code.windows(5).find(|b| b[0] == 0x40).unwrap();
        assert_eq!(&operand[1..], &expected_offset.to_be_bytes());
        assert_eq!(
            code,
            Program::parse(&assembly.to_text())
                .unwrap()
                .encode_with_literals(&BTreeMap::new(), &BTreeMap::new(), &arrays)
                .unwrap()
        );
    }
    for source in [
        "function f(n) as Toybox.Lang.Array<Toybox.Lang.Number> { return [n, 1]; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number> { return [1 + 2]; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number> { return [(true || false).hashCode()]; }",
    ] {
        let f = compile(source).unwrap();
        assert!(!f.to_ir_text().contains(".array-constant"));
        assert!(f.to_ir_text().contains(".array-begin"));
    }
}

#[test]
fn constant_array_ir_rejects_bad_elements_and_checks_api_and_use_contracts() {
    let text = "ciq-ir\n.function f\n.array-constant %0 number 0x00000001\n.return %0\n.end\n";
    let f = parse_ir(text).unwrap();
    assert!(matches!(
        f.operations()[0].operation,
        Operation::ArrayConstant {
            element_type: ArrayElement::Number,
            ..
        }
    ));
    for bad in [
        ".array-constant",
        ".array-constant %0",
        ".array-constant %0 number",
        ".array-constant %0 view 0x00000001",
        ".array-constant %0 number 1",
        ".array-constant %0 number 0x100000000",
        ".array-constant %0 number 0xgggggggg",
        ".array-constant %0 number %1",
    ] {
        assert!(
            parse_ir(&text.replace(".array-constant %0 number 0x00000001", bad)).is_err(),
            "accepted {bad}"
        );
    }
    assert!(parse_ir(&text.replace(".return %0", ".return %1")).is_err());
    let old = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(1, 0, 0),
        ciq_compiler::api::ApiLevel(1, 0, 0),
    )
    .unwrap();
    assert!(ciq_compiler::parse_ir_with_api(text, old).is_ok());
    assert!(ciq_compiler::parse_ir_with_api(&text.replace("number", "char"), old).is_err());
}

#[test]
fn array_requirements_reject_missing_ambiguous_conflicting_and_mispinned_records() {
    let assembly = Program::parse(
        ".array required\n.element number=0x00000001\n.endarray\napush &required\nreturn\n",
    )
    .unwrap();
    let record = ".array placed\n.element number=0x00000001\n.endarray\n";
    let data = Source::parse(record).unwrap();
    assert_eq!(array_bindings(&assembly, &data).unwrap()["required"], 0);
    for text in [
        "".to_owned(),
        record.replace("01", "02"),
        format!("{record}{}", record.replace("placed", "second")),
        format!("{record}.array required\n.endarray\n"),
    ] {
        assert!(array_bindings(&assembly, &Source::parse(&text).unwrap()).is_err());
    }
    let pinned = Program::parse(
        &assembly
            .to_text()
            .replace(".array required", ".array required at=0x00000000"),
    )
    .unwrap();
    let bindings = array_bindings(&pinned, &data).unwrap();
    assert!(!bindings.contains_key("required"));
    assert_eq!(
        pinned
            .encode_with_literals(&BTreeMap::new(), &BTreeMap::new(), &bindings)
            .unwrap(),
        [0x40, 0, 0, 0, 0, 0x16]
    );
    let shifted = Source::parse(&format!(".string before \"prefix\"\n{record}")).unwrap();
    assert!(array_bindings(&pinned, &shifted).is_err());
}

#[test]
fn pooled_array_requirements_match_contents_independently_of_local_aliases() {
    let assembly = Program::parse(".array required\n.string item \"one\"\n.element string=&item\n.endarray\napush &required\nreturn\n").unwrap();
    assert_eq!(assembly, Program::parse(&assembly.to_text()).unwrap());
    let text = ".array actual\n.string different \"one\"\n.element string=&different\n.endarray\n";
    let data = Source::parse(text).unwrap();
    let bindings = array_bindings(&assembly, &data).unwrap();
    assert_eq!(bindings["required"], 0);
    assert_eq!(
        assembly
            .encode_with_literals(&BTreeMap::new(), &BTreeMap::new(), &bindings)
            .unwrap(),
        [0x40, 0, 0, 0, 0, 0x16]
    );
    assert!(
        array_bindings(
            &assembly,
            &Source::parse(&text.replace("one", "two")).unwrap()
        )
        .is_err()
    );
    // Identical alias spelling must not hide different String contents.
    assert!(
        array_bindings(
            &assembly,
            &Source::parse(&text.replace("different", "item").replace("one", "two")).unwrap()
        )
        .is_err()
    );
    let ambiguous = format!("{text}{}", text.replace("actual", "other"));
    assert!(array_bindings(&assembly, &Source::parse(&ambiguous).unwrap()).is_err());
}
