use ciq_assembly::Program;
use ciq_compiler::{Function, Operation, compile, parse_ir, symbol_bindings};
use ciq_mc::{Opcode, decode};
use std::collections::BTreeMap;

#[derive(Debug)]
enum Stack {
    Integer(i32),
    Callable(u32),
    Context,
}
fn integer(value: Stack) -> i32 {
    let Stack::Integer(value) = value else {
        panic!("expected integer")
    };
    value
}
fn evaluate(bytes: &[u8], parameters: &[i32]) -> (i32, Vec<u32>) {
    // A restricted frame/call check with synthetic callees; Garmin is the oracle.
    let mut slots: BTreeMap<u8, i32> = parameters
        .iter()
        .enumerate()
        .map(|(index, value)| (index as u8 + 1, *value))
        .collect();
    let mut stack = Vec::new();
    let mut calls = Vec::new();
    for located in decode(bytes).unwrap() {
        let instruction = located.instruction;
        match instruction.opcode() {
            Opcode::Argc | Opcode::Argcincsp => {
                assert_eq!(instruction.operands()[0] as usize, parameters.len() + 1)
            }
            Opcode::Ipushz => stack.push(Stack::Integer(0)),
            Opcode::Ipush1 => stack.push(Stack::Integer(instruction.operands()[0] as i8 as i32)),
            Opcode::Lgetv => stack.push(Stack::Integer(slots[&instruction.operands()[0]])),
            Opcode::Lputv => {
                slots.insert(instruction.operands()[0], integer(stack.pop().unwrap()));
            }
            Opcode::Getmv => {
                assert_eq!(
                    u32::from_be_bytes(instruction.operands()[..4].try_into().unwrap()),
                    2
                );
                stack.push(Stack::Callable(u32::from_be_bytes(
                    instruction.operands()[4..].try_into().unwrap(),
                )));
            }
            Opcode::Frpush => stack.push(Stack::Context),
            Opcode::Invokem | Opcode::Invokemz => {
                let count = if instruction.opcode() == Opcode::Invokemz {
                    0
                } else {
                    instruction.operands()[0] as usize - 1
                };
                let arguments: Vec<_> = stack
                    .split_off(stack.len() - count)
                    .into_iter()
                    .map(integer)
                    .collect();
                if count != 0 {
                    assert!(matches!(stack.pop(), Some(Stack::Context)));
                }
                let Some(Stack::Callable(target)) = stack.pop() else {
                    panic!("missing callable")
                };
                calls.push(target);
                let value = match target {
                    0x80 => {
                        assert_eq!(arguments.len(), 2);
                        arguments[0] - arguments[1]
                    }
                    0x81 => {
                        assert_eq!(arguments.len(), 1);
                        arguments[0] + 3
                    }
                    0x82 => {
                        assert!(arguments.is_empty());
                        40 + calls.len() as i32
                    }
                    0x83 => {
                        assert!(arguments.is_empty());
                        2
                    }
                    _ => panic!("unknown synthetic callee"),
                };
                stack.push(Stack::Integer(value));
            }
            Opcode::Addv | Opcode::Subv => {
                let right = integer(stack.pop().unwrap());
                let left = integer(stack.pop().unwrap());
                stack.push(Stack::Integer(if instruction.opcode() == Opcode::Addv {
                    left + right
                } else {
                    left - right
                }));
            }
            Opcode::Return => {
                assert_eq!(stack.len(), 1);
                return (integer(stack.pop().unwrap()), calls);
            }
            other => panic!("unexpected opcode {other:?}"),
        }
    }
    panic!("missing return")
}
fn symbols() -> Program {
    Program::parse(".symbol m \"globals/Numbers\" id=0x02\n.symbol subtract \"subtract\" id=0x80\n.symbol adjust \"adjust\" id=0x81\n.symbol next \"next\" id=0x82\n.symbol two \"two\" id=0x83\n").unwrap()
}
fn check(source: &str, arguments: &[i32], expected: i32, expected_calls: &[u32]) {
    let ir = compile(source).unwrap();
    let native = ir.lower();
    let bindings = symbol_bindings(&native, &symbols()).unwrap();
    let bytes = native
        .encode_with_references(&bindings, &Default::default())
        .unwrap();
    assert_eq!(
        evaluate(&bytes, arguments),
        (expected, expected_calls.to_vec())
    );
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
fn parameter_reads_and_local_slots_survive_all_pipelines() {
    check(
        "function difference(first, second) { var adjusted = first + 3; return adjusted - second; }",
        &[45, 2],
        46,
        &[],
    );
    check(
        "function repeat(value) { return value + value; }",
        &[-3],
        -6,
        &[],
    );
    check("function ignore(value) { return 41; }", &[99], 41, &[]);
    let ir = compile("function difference(a,b) { var x = a + 3; return x - b; }").unwrap();
    assert_eq!(ir.parameter_count(), 2);
    assert_eq!(
        ir.lower().encode().unwrap(),
        [
            0x4a, 3, 1, 0x12, 1, 0x38, 3, 3, 0x13, 3, 0x12, 3, 0x12, 2, 4, 0x16
        ]
    );
}

#[test]
fn argument_order_and_nested_call_contexts_are_preserved() {
    check(
        "function probe() { return Numbers.subtract(40 + 5, 2); }",
        &[],
        43,
        &[0x80],
    );
    check(
        "function probe() { return Numbers.adjust(40); }",
        &[],
        43,
        &[0x81],
    );
    check(
        "function probe() { var saved = Numbers.next(); return Numbers.subtract(saved + 1, Numbers.two()) + 1; }",
        &[],
        41,
        &[0x82, 0x83, 0x80],
    );
    check(
        "function probe() { return Numbers.subtract(Numbers.next(), Numbers.next()); }",
        &[],
        -1,
        &[0x82, 0x82, 0x80],
    );
    check(
        "function probe() { return Numbers.adjust(Numbers.subtract(Numbers.next(), Numbers.two())); }",
        &[],
        42,
        &[0x82, 0x83, 0x80, 0x81],
    );
    check(
        "function probe(a) { return Numbers.subtract(a, Numbers.adjust(2)); }",
        &[10],
        5,
        &[0x81, 0x80],
    );
}

#[test]
fn parameter_positions_and_argument_dependencies_are_visible_in_ir() {
    let ir = compile("function probe(a,b) { return Numbers.subtract(b,a); }").unwrap();
    assert!(matches!(
        ir.operations()[0].operation,
        Operation::ParameterRead { index: 1, .. }
    ));
    assert!(ir.to_ir_text().contains(".function probe parameters=2"));
    assert!(ir.to_ir_text().contains(".parameter %0 1"));
    assert!(ir.to_ir_text().contains(".call %2 Numbers subtract %0 %1"));
    assert_eq!(parse_ir(&ir.to_ir_text()).unwrap(), ir);
    assert!(parse_ir("ciq-ir\n.function probe\n.parameter %0 0\n.return %0\n.end").is_err());
    assert!(
        parse_ir("ciq-ir\n.function probe parameters=1\n.parameter %0 1\n.return %0\n.end")
            .is_err()
    );
    assert!(
        parse_ir(
            "ciq-ir\n.function probe\n.integer %0 40\n.call %1 Numbers adjust %9\n.return %1\n.end"
        )
        .is_err()
    );
    assert!(parse_ir("ciq-ir\n.function probe\n.call %0 Numbers next\n.call %1 Numbers two\n.call %2 Numbers subtract %1 %0\n.return %2\n.end").is_err());
    let typed = Function::with_parameters("probe".into(), 0, ir.operations().to_vec());
    assert!(typed.is_err());
}

#[test]
fn malformed_parameter_and_argument_lists_are_rejected() {
    for source in [
        "function probe(a,a) { return a; }",
        "function probe(a,) { return a; }",
        "function probe(a as Number) { return a; }",
        "function probe(a=1) { return a; }",
        "function probe(a) { var a = 2; return a; }",
        "function probe(Numbers) { return Numbers.adjust(1); }",
        "function probe() { return Numbers.adjust(,1); }",
        "function probe() { return Numbers.adjust(1,); }",
        "function probe() { return Numbers.subtract(1 2); }",
        "function probe() { return Numbers.adjust(missing); }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    for header in ["parameters=-1", "parameters=255", "parameters=", "arity=2"] {
        assert!(
            parse_ir(&format!(
                "ciq-ir\n.function probe {header}\n.integer %0 41\n.return %0\n.end"
            ))
            .is_err()
        );
    }
}

#[test]
fn signature_call_frame_and_nesting_limits_are_enforced() {
    let parameters = (0..254)
        .map(|index| format!("p{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let ir = compile(&format!(
        "function probe({parameters}) {{ var x = p253; return x; }}"
    ))
    .unwrap();
    assert_eq!(
        ir.lower().encode().unwrap(),
        [0x4a, 255, 1, 0x12, 254, 0x13, 255, 0x12, 255, 0x16]
    );
    assert!(
        compile(&format!(
            "function probe({parameters},extra) {{ return 1; }}"
        ))
        .is_err()
    );
    assert!(
        compile(&format!(
            "function probe({parameters}) {{ var x = 1; var y = 2; return y; }}"
        ))
        .is_err()
    );
    let arguments = vec!["1"; 254].join(",");
    let ir = compile(&format!(
        "function probe() {{ return Numbers.adjust({arguments}); }}"
    ))
    .unwrap();
    assert!(ir.assembly().contains("invokem 0xff"));
    assert!(
        compile(&format!(
            "function probe() {{ return Numbers.adjust({arguments},1); }}"
        ))
        .is_err()
    );
    let nested = format!("{}1{}", "Numbers.adjust(".repeat(64), ")".repeat(64));
    assert!(compile(&format!("function probe() {{ return {nested}; }}")).is_ok());
    let nested = format!("Numbers.adjust({nested})");
    assert!(
        compile(&format!("function probe() {{ return {nested}; }}"))
            .unwrap_err()
            .message
            .contains("nesting")
    );
    let body = (0..254)
        .map(|n| format!(".integer %{n} 1\n"))
        .collect::<String>();
    let args = (0..254).map(|n| format!(" %{n}")).collect::<String>();
    assert!(
        parse_ir(&format!(
            "ciq-ir\n.function probe\n{body}.call %254 Numbers adjust{args}\n.return %254\n.end"
        ))
        .is_ok()
    );
    let body = format!("{body}.integer %254 1\n");
    assert!(parse_ir(&format!("ciq-ir\n.function probe\n{body}.call %255 Numbers adjust{args} %254\n.return %255\n.end")).is_err());
}
