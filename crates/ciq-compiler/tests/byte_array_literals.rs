use ciq_assembly::{Item, Program};
use ciq_compiler::{
    Operation,
    api::{ApiContext, ApiLevel},
    byte_array_bindings, compile_function, compile_function_with_api, parse_function_ir,
    parse_function_ir_with_api,
};
use ciq_prg::data::{Fragment, Source};
use std::collections::BTreeMap;

#[test]
fn byte_literal_constants_keep_exact_bytes_and_typed_bindings() {
    for (literal, bytes) in [
        (
            "[0, 1, 127, 128, 255, -1, -128, 'A']b",
            vec![0, 1, 127, 128, 255, 255, 128, 65],
        ),
        ("[]b", vec![]),
    ] {
        let f = compile_function(&format!(
            "function f() as Toybox.Lang.ByteArray {{ return {literal}; }}"
        ))
        .unwrap();
        assert!(f.operations().iter().any(|o| matches!(&o.operation, Operation::ByteArrayConstant { values, .. } if values == &bytes)));
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        let p = f.lower();
        assert_eq!(p, Program::parse(&p.to_text()).unwrap());
        let data = Source {
            fragments: p
                .items()
                .iter()
                .filter_map(|item| {
                    if let Item::ByteArray(d) = item {
                        Some(Fragment::ByteArray(d.clone()))
                    } else {
                        None
                    }
                })
                .collect(),
        };
        let bindings = byte_array_bindings(&p, &data).unwrap();
        let empty = BTreeMap::new();
        let code = p
            .encode_with_byte_arrays(&empty, &empty, &empty, &empty, &bindings)
            .unwrap();
        assert!(
            ciq_mc::decode(&code)
                .unwrap()
                .iter()
                .any(|i| i.instruction.opcode() == ciq_mc::Opcode::Bpush)
        );
        assert!(
            !f.operations()
                .iter()
                .any(|o| matches!(o.operation, Operation::ByteArrayBegin { .. }))
        );
        let wire = data.encode().unwrap().0;
        assert_eq!(
            ciq_prg::data::byte_arrays::byte_array_at(&wire, 0)
                .unwrap()
                .0
                .values,
            bytes
        );
    }
}

#[test]
fn computed_literals_use_private_initialization_without_truncating_bad_tokens() {
    for source in [
        "function f(n) as Toybox.Lang.ByteArray { return [n, n + 1]b; }",
        "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.ByteArray { return [1, flag ? 2 : 3, 'A']b; }",
        "function f() as Toybox.Lang.ByteArray { return [256]b; }",
        "function f() as Toybox.Lang.ByteArray { return [-129]b; }",
        "function f() as Toybox.Lang.ByteArray { return ['é']b; }",
        "function f() as Toybox.Lang.ByteArray { return [1 + 2]b; }",
    ] {
        let f = compile_function(source).unwrap();
        assert!(
            f.operations()
                .iter()
                .any(|o| matches!(o.operation, Operation::ByteArrayBegin { .. }))
        );
        assert!(
            !f.operations()
                .iter()
                .any(|o| matches!(o.operation, Operation::ByteArrayConstant { .. }))
        );
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        let p = f.lower();
        assert_eq!(p, Program::parse(&p.to_text()).unwrap());
        let code = p.encode().unwrap();
        assert!(
            ciq_mc::decode(&code)
                .unwrap()
                .iter()
                .any(|i| i.instruction.opcode() == ciq_mc::Opcode::Newba)
        );
    }
    for source in [
        "function f() { var a = [true]b; }",
        "function f() { var a = [null]b; }",
        "function f() { var a = [1l]b; }",
        "function f() { var a = [1.0]b; }",
        "function f() { var a = [\"a\"]b; }",
        "function f() { var a = [[1]b]b; }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
}

const BUILDER: &str = "ciq-ir\n.function f result=byte-array\n.byte-array-begin &0 2\n.integer %0 7\n.char %1 0x00000041\n.array-fill-many &0 0 %0 %1\n.array-finish %2 &0\n.return %2\n.end\n";

#[test]
fn builder_completion_type_and_api_checks_apply_to_native_and_saved_ir() {
    let f = parse_function_ir(BUILDER).unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    let code = f.lower().encode().unwrap();
    assert!(
        ciq_mc::decode(&code)
            .unwrap()
            .iter()
            .any(|i| i.instruction.opcode() == ciq_mc::Opcode::Aputvdup)
    );
    for bad in [
        BUILDER.replace("&0 2", "&0 0"),
        BUILDER.replace(".char %1 0x00000041", ".boolean %1 true"),
        BUILDER.replace(".array-fill-many &0 0 %0 %1", ".array-fill-many &0 1 %0 %1"),
        BUILDER.replace(".array-fill-many &0 0 %0 %1", ".array-fill-many &0 0 %0"),
        BUILDER.replace(
            ".array-fill-many &0 0 %0 %1",
            ".array-fill-many &0 0 %0 %1 %0",
        ),
        BUILDER.replace(".array-fill-many &0 0 %0 %1", ".dictionary-fill &0 0 %0 %1"),
        BUILDER.replace(
            ".byte-array-begin &0 2",
            ".byte-array-begin &0 2\n.array-begin &0 number 2",
        ),
        BUILDER.replace(".array-finish %2 &0", ".array-finish %2 &99"),
    ] {
        assert!(parse_function_ir(&bad).is_err(), "{bad}");
    }
    let old = ApiContext::new(ApiLevel(2, 4, 0), ApiLevel(2, 4, 0)).unwrap();
    let current = ApiContext::new(ApiLevel(3, 0, 0), ApiLevel(3, 0, 0)).unwrap();
    for source in [
        "function f() as Void { var a = []b; }",
        "function f(n) as Void { var a = [n]b; }",
    ] {
        let f = compile_function_with_api(source, current).unwrap();
        assert_eq!(
            f,
            parse_function_ir_with_api(&f.to_ir_text(), current).unwrap()
        );
        assert!(compile_function_with_api(source, old).is_err());
        assert!(parse_function_ir_with_api(&f.to_ir_text(), old).is_err());
    }
    for payload in ["0x0", "0xgg", "ff", "0x000"] {
        let ir = format!(
            "ciq-ir\n.function f result=byte-array\n.byte-array-constant %0 {payload}\n.return %0\n.end\n"
        );
        assert!(parse_function_ir(&ir).is_err());
    }
}

#[test]
fn literal_binding_requires_unique_payloads_and_respects_pins() {
    let p =
        Program::parse(".bytearray literal\n.bytes 0708\n.endbytearray\nbpush &literal\nreturn")
            .unwrap();
    let data = Source::parse(".bytearray stored\n.bytes 0708\n.endbytearray").unwrap();
    assert_eq!(byte_array_bindings(&p, &data).unwrap()["literal"], 0);
    let missing = Source::parse(".bytearray stored\n.bytes 0709\n.endbytearray").unwrap();
    assert!(byte_array_bindings(&p, &missing).is_err());
    let duplicate = Source::parse(
        ".bytearray a\n.bytes 0708\n.endbytearray\n.bytearray b\n.bytes 0708\n.endbytearray",
    )
    .unwrap();
    assert!(byte_array_bindings(&p, &duplicate).is_err());
    let pinned = Program::parse(
        &p.to_text()
            .replace(".bytearray literal", ".bytearray literal at=0x0"),
    )
    .unwrap();
    let map = byte_array_bindings(&pinned, &duplicate).unwrap();
    assert!(!map.contains_key("literal"));
    assert!(
        pinned
            .encode_with_byte_arrays(
                &BTreeMap::new(),
                &BTreeMap::new(),
                &BTreeMap::new(),
                &BTreeMap::new(),
                &map
            )
            .is_ok()
    );
    let direct = Program::parse("bpush &stored\nreturn").unwrap();
    assert_eq!(byte_array_bindings(&direct, &data).unwrap()["stored"], 0);
}
