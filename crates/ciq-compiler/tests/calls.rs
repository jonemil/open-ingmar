use ciq_assembly::Program;
use ciq_compiler::{Operation, compile, parse_ir, symbol_bindings};
use ciq_mc::{Opcode, decode};
use std::collections::BTreeMap;

fn symbols() -> Program {
    // The short module field and actual globals module reference are distinct.
    Program::parse(".symbol field \"Numbers\" id=0x00000099\n.symbol module_record \"globals/Numbers\" id=0x00000002\n.symbol left \"left\" id=0x00000084\n.symbol right \"right\" id=0x00000085\n").unwrap()
}

fn evaluate(bytes: &[u8]) -> (i32, Vec<u32>) {
    // Synthetic integer callees with observable order, not a VM/frame emulator.
    let mut stack = Vec::new();
    let mut slots = BTreeMap::new();
    let mut callable = None;
    let mut calls = Vec::new();
    for located in decode(bytes).unwrap() {
        let instruction = located.instruction;
        match instruction.opcode() {
            Opcode::Argc | Opcode::Argcincsp => assert_eq!(instruction.operands()[0], 1),
            Opcode::Ipushz => stack.push(0),
            Opcode::Ipush1 => stack.push(instruction.operands()[0] as i32),
            Opcode::Getmv => {
                assert_eq!(
                    u32::from_be_bytes(instruction.operands()[..4].try_into().unwrap()),
                    2
                );
                let target = u32::from_be_bytes(instruction.operands()[4..].try_into().unwrap());
                assert!(callable.replace(target).is_none());
            }
            Opcode::Invokemz => {
                let target = callable.take().unwrap();
                calls.push(target);
                // Incrementing per call makes dropped/duplicated calls observable.
                stack.push(if target == 0x84 { 40 } else { 80 } + calls.len() as i32);
            }
            Opcode::Lputv => {
                slots.insert(instruction.operands()[0], stack.pop().unwrap());
            }
            Opcode::Lgetv => stack.push(slots[&instruction.operands()[0]]),
            Opcode::Addv | Opcode::Subv => {
                let right = stack.pop().unwrap();
                let left = stack.pop().unwrap();
                stack.push(if instruction.opcode() == Opcode::Addv {
                    left + right
                } else {
                    left - right
                });
            }
            Opcode::Return => {
                assert_eq!(stack.len(), 1);
                return (stack[0], calls);
            }
            other => panic!("unexpected opcode {other:?}"),
        }
    }
    panic!("missing return")
}

fn check(body: &str, expected: i32, calls: &[u32]) {
    let ir = compile(&format!("function probe() {{ {body} }}")).unwrap();
    let native = ir.lower();
    assert!(
        native.encode().is_err(),
        "external symbols require explicit binding"
    );
    let bindings = symbol_bindings(&native, &symbols()).unwrap();
    let bytes = native
        .encode_with_references(&bindings, &Default::default())
        .unwrap();
    assert_eq!(evaluate(&bytes), (expected, calls.to_vec()));
    let resumed = parse_ir(&ir.to_ir_text()).unwrap();
    assert_eq!(resumed, ir);
    for model in [resumed.lower(), Program::parse(&native.to_text()).unwrap()] {
        assert_eq!(model, native);
        assert_eq!(
            model
                .encode_with_references(&bindings, &Default::default())
                .unwrap(),
            bytes
        );
    }
}

#[test]
fn calls_preserve_source_order_and_local_initialization_across_pipelines() {
    check("return Numbers.left();", 41, &[0x84]);
    check(
        "return Numbers.left() - Numbers.right();",
        -41,
        &[0x84, 0x85],
    );
    check(
        "return Numbers.right() - Numbers.left();",
        39,
        &[0x85, 0x84],
    );
    check(
        "var saved = Numbers.left(); return saved + saved;",
        82,
        &[0x84],
    );
    check(
        "var unused = Numbers.right(); var saved = Numbers.left(); return saved + Numbers.left();",
        85,
        &[0x85, 0x84, 0x84],
    );
    check(
        "return Numbers.left() - (Numbers.right() - Numbers.left());",
        2,
        &[0x84, 0x85, 0x84],
    );
}

#[test]
fn call_names_spans_and_zero_argument_encoding_are_explicit() {
    let source = "function probe() { return Numbers.left(); }";
    let ir = compile(source).unwrap();
    assert!(
        matches!(&ir.operations()[0].operation, Operation::ModuleCall { module, member, .. } if module == "Numbers" && member == "left")
    );
    let span = ir.operations()[0].span.unwrap();
    assert_eq!(&source[span.start..span.end], "Numbers.left()");
    let model = ir.lower();
    let bindings = symbol_bindings(&model, &symbols()).unwrap();
    assert_eq!(bindings["__ciq_module_Numbers"], 2);
    assert_eq!(bindings["__ciq_member_left"], 0x84);
    assert_eq!(
        model
            .encode_with_references(&bindings, &Default::default())
            .unwrap(),
        [0x35, 1, 0x45, 0, 0, 0, 2, 0, 0, 0, 0x84, 0x48, 0x16]
    );
}

#[test]
fn incompatible_effectful_ir_orders_are_rejected_but_pure_orders_still_work() {
    for body in [
        ".call %0 Numbers.left\n.call %1 Numbers.right\n.subtract %2 %1 %0\n.return %2", // wrong arity below also tested
        ".call %0 Numbers left\n.call %1 Numbers right\n.subtract %2 %1 %0\n.return %2", // reverses calls
        ".call %0 Numbers left\n.integer %1 1\n.subtract %2 %1 %0\n.return %2", // would move pure computation across call
        ".call %0 Numbers left\n.integer %1 1\n.local $0 %1\n.return %0", // moves call across store
    ] {
        assert!(
            parse_ir(&format!("ciq-ir\n.function probe\n{body}\n.end")).is_err(),
            "{body}"
        );
    }
    let text = "ciq-ir\n.function probe\n.call %0 Numbers left\n.call %1 Numbers right\n.subtract %2 %0 %1\n.return %2\n.end";
    let ir = parse_ir(text).unwrap();
    assert_eq!(parse_ir(&ir.to_ir_text()).unwrap(), ir);
    assert!(parse_ir("ciq-ir\n.function probe\n.integer %0 8\n.integer %1 40\n.subtract %2 %1 %0\n.return %2\n.end").is_ok());
}

#[test]
fn unsupported_call_shapes_are_rejected_with_source_diagnostics() {
    for body in [
        "return left();",
        "return Numbers.left(1,);",
        "return Numbers.Inner.left();",
        "return Numbers.left;",
        "return Numbers.return();",
        "return Numbers.left()();",
        "var Numbers = 1; return Numbers.left();",
    ] {
        assert!(
            compile(&format!("function probe() {{ {body} }}")).is_err(),
            "{body}"
        );
    }
    let error = compile("function probe() {\n  return Numbers.left(1,);\n}").unwrap_err();
    assert_eq!((error.line, error.column), (2, 25));
    assert!(error.message.contains("expected"));
    for call in [
        ".call %0 Numbers.left",
        ".call %0 bad.module left",
        ".call %0 Numbers 42",
    ] {
        assert!(
            parse_ir(&format!(
                "ciq-ir\n.function probe\n{call}\n.return %0\n.end"
            ))
            .is_err()
        );
    }
}

#[test]
fn symbol_binding_rejects_missing_ambiguous_or_non_symbol_sources() {
    let code = compile("function probe() { return Numbers.left(); }")
        .unwrap()
        .lower();
    for source in [
        ".symbol x \"Numbers\" id=0x99\n.symbol y \"left\" id=0x84", // field is not module
        ".symbol x \"globals/Numbers\" id=0x02",                     // missing member
        ".symbol x \"globals/Numbers\" id=0x02\n.symbol a \"left\" id=0x84\n.symbol b \"left\" id=0x85", // ambiguous
        ".symbol x \"globals/Numbers\" id=0x02\n.symbol a \"left\" id=0x02", // duplicate id
        ".symbol x \"globals/Numbers\" id=0x02\n.symbol x \"left\" id=0x84", // duplicate alias
        ".symbol x \"globals/Numbers\" id=0x02\n.symbol a \"left\" id=0x84\nreturn", // not metadata-only
        ".symbol __ciq_module_Numbers \"wrong\" id=0x99\n.symbol x \"globals/Numbers\" id=0x02\n.symbol a \"left\" id=0x84", // conflicting reserved alias
    ] {
        assert!(
            symbol_bindings(&code, &Program::parse(source).unwrap()).is_err(),
            "{source}"
        );
    }
    let ordinary = Program::parse("getselfv @left\ninvokemz").unwrap();
    let bindings = symbol_bindings(&ordinary, &symbols()).unwrap();
    assert!(
        ordinary
            .encode_with_references(&bindings, &Default::default())
            .is_ok()
    );
}
