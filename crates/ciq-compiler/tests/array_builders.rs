use ciq_compiler::{compile, parse_ir};
use ciq_mc::{Opcode, decode};

const BUILD: &str = "ciq-ir\n.function make\n.array-begin &0 number 2\n.integer %0 10\n.array-fill &0 0 %0\n.integer %1 20\n.array-fill &0 1 %1\n.array-finish %2 &0\n.return %2\n.end\n";

#[test]
fn grouped_fills_retain_the_array_and_recheck_range_type_and_value_usage() {
    let grouped = BUILD
        .replace(".array-fill &0 0 %0\n", "")
        .replace(".array-fill &0 1 %1", ".array-fill-many &0 0 %0 %1");
    let f = parse_ir(&grouped).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let code = decode(&f.lower().encode().unwrap()).unwrap();
    assert_eq!(
        code.iter()
            .filter(|i| i.instruction.opcode() == Opcode::Aputvdup)
            .count(),
        1
    );
    assert_eq!(
        code.iter()
            .filter(|i| i.instruction.opcode() == Opcode::Aputv)
            .count(),
        1
    );
    for text in [
        grouped.replace("&0 0 %0 %1", "&0 1 %0 %1"),
        grouped.replace("&0 0 %0 %1", "&0 0 %0 %1 %0"),
        grouped.replace("&0 0 %0 %1", "&0 0 %0"),
        grouped.replace("&0 0 %0 %1", "&0 0"),
        grouped.replace("&0 0 %0 %1", "&0 0 %0 %99"),
        grouped.replace(".integer %1 20", ".boolean %1 true"),
    ] {
        assert!(parse_ir(&text).is_err(), "accepted {text}");
    }
}

#[test]
fn builders_allocate_final_length_fill_directly_and_publish_only_when_complete() {
    let f = parse_ir(BUILD).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let bytes = f.lower().encode().unwrap();
    assert_eq!(
        bytes,
        [
            0x4a, 1, 1, 0x38, 2, 0x14, 0x13, 1, 0x12, 1, 0x37, 0x38, 10, 0x11, 0x12, 1, 0x38, 1,
            0x38, 20, 0x11, 0x12, 1, 0x16
        ]
    );
    assert_eq!(
        ciq_assembly::Program::parse(&f.lower().to_text())
            .unwrap()
            .encode()
            .unwrap(),
        bytes
    );
    let source = compile(
        "function f(value) as Toybox.Lang.Array<Toybox.Lang.Number> { return [value, 2]; }",
    )
    .unwrap();
    assert!(source.to_ir_text().contains(".array-begin &0 number 2"));
    assert!(!source.to_ir_text().contains(".call-native"));
    let bytes = source.lower().encode().unwrap();
    assert_eq!(
        decode(&bytes)
            .unwrap()
            .iter()
            .filter(|i| matches!(i.instruction.opcode(), Opcode::Aputv | Opcode::Aputvdup))
            .count(),
        2
    );
}

#[test]
fn malformed_builder_ir_cannot_publish_or_access_uninitialized_slots() {
    for bad in [
        BUILD.replace("number 2", "number 0"),
        BUILD.replace("number 2", "number 2147483648"),
        BUILD.replace("number 2", "number -1"),
        BUILD.replace(".array-fill &0 0 %0", ".array-fill &0 1 %0"),
        BUILD.replace(".array-fill &0 1 %1", ".array-fill &0 0 %1"),
        BUILD.replace(".array-fill &0 1 %1", ".array-fill &0 2 %1"),
        BUILD.replace(".array-fill &0 1 %1\n", ".discard %1\n"),
        BUILD.replace(".array-begin &0 number 2\n", ""),
        BUILD.replace(".array-finish %2 &0", ".read %2 $0"),
        BUILD.replace(".array-finish %2 &0", ".array-finish %2 &1"),
        BUILD.replace(".integer %0 10", ".boolean %0 true"),
        BUILD.replace(".return %2", ".discard %2\n.array-finish %3 &0\n.return %3"),
        BUILD.replace(".array-finish %2 &0\n.return %2", ".return-void"),
        BUILD.replace(".array-fill &0 0 %0", ".array-fill &0 0 %99"),
        BUILD.replace(".integer %0 10", ".array-begin &0 number 2\n.integer %0 10"),
    ] {
        assert!(parse_ir(&bad).is_err(), "accepted {bad}");
    }
}

#[test]
fn builder_progress_must_agree_at_joins_and_loops_can_restart_completed_builders() {
    let joined = "ciq-ir\n.function f\n.block ^0\n.array-begin &0 number 1\n.boolean %0 true\n.branch %0 ^1 ^2\n.block ^1\n.integer %1 10\n.array-fill &0 0 %1\n.jump ^3\n.block ^2\n.integer %2 20\n.array-fill &0 0 %2\n.jump ^3\n.block ^3\n.array-finish %3 &0\n.return %3\n.end\n";
    let f = parse_ir(joined).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let bad = joined.replace(".array-fill &0 0 %2", ".discard %2");
    assert!(parse_ir(&bad).is_err());
    let source = "function f(n) { var count = n; var total = 0; while (count > 0) { var a = [count, (true || false).hashCode()]; total += a[0]; count -= 1; } return total; }";
    let f = compile(source).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let dangling = joined.replace(
        ".array-finish %3 &0\n.return %3",
        ".integer %3 1\n.return %3",
    );
    assert!(parse_ir(&dangling).is_err());
}

#[test]
fn private_builders_count_toward_the_frame_budget_and_have_separate_identities() {
    let boundary = BUILD.replace(".function make", ".function make parameters=254");
    let f = parse_ir(&boundary).unwrap();
    let code = decode(&f.lower().encode().unwrap()).unwrap();
    assert_eq!(code[0].instruction.operands(), [255, 1]);
    let overflow = boundary.replace(
        ".array-begin",
        ".integer %99 3\n.local $0 %99\n.array-begin",
    );
    assert!(
        parse_ir(&overflow)
            .unwrap_err()
            .message
            .contains("255-slot")
    );
    // Ordinary local $0 is separate from builder &0, despite the same number.
    let f = parse_ir(&BUILD.replace(
        ".array-begin",
        ".integer %99 3\n.local $0 %99\n.array-begin",
    ))
    .unwrap();
    let code = decode(&f.lower().encode().unwrap()).unwrap();
    assert_eq!(code[0].instruction.operands(), [1, 2]);
}
