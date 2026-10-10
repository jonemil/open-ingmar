use ciq_compiler::{Function, compile, parse_ir};
use ciq_mc::{Opcode, decode};

// A bounded integer/Boolean instruction oracle, independent of frontend/IR traversal.
// Calls and strings still require official simulator evidence.
fn execute(function: &Function, argument: i32) -> i32 {
    let bytes = function.lower().encode().unwrap();
    let code = decode(&bytes).unwrap();
    let mut stack = Vec::<i32>::new();
    let mut locals = vec![0; 256];
    locals[1] = argument;
    let mut pc = 0;
    for _ in 0..4096 {
        let located = code
            .iter()
            .find(|i| i.offset == pc)
            .expect("instruction target");
        let ins = &located.instruction;
        pc += ins.encoded_len();
        match ins.opcode() {
            Opcode::Argc | Opcode::Argcincsp => (),
            Opcode::Ipushz => stack.push(0),
            Opcode::Ipush1 => stack.push(ins.operands()[0] as i32),
            Opcode::Lgetv => stack.push(locals[ins.operands()[0] as usize]),
            Opcode::Lputv => locals[ins.operands()[0] as usize] = stack.pop().unwrap(),
            Opcode::Addv | Opcode::Subv | Opcode::Eq | Opcode::Gt => {
                let right = stack.pop().unwrap();
                let left = stack.pop().unwrap();
                stack.push(match ins.opcode() {
                    Opcode::Addv => left + right,
                    Opcode::Subv => left - right,
                    Opcode::Eq => i32::from(left == right),
                    _ => i32::from(left > right),
                });
            }
            Opcode::Bf => {
                if stack.pop().unwrap() == 0 {
                    pc = located.relative_target().unwrap() as usize;
                }
            }
            Opcode::Goto => pc = located.relative_target().unwrap() as usize,
            Opcode::Return => {
                let value = stack.pop().unwrap();
                assert!(stack.is_empty());
                return value;
            }
            other => panic!("unsupported oracle instruction {other:?}"),
        }
    }
    panic!("bounded program did not return")
}

#[test]
fn conditions_choose_one_arm_with_common_locals_and_preserve_all_pipeline_stages() {
    let source = include_str!("../../../examples/compiler-probe/conditions.mc");
    let f = compile(source).unwrap();
    assert_eq!(
        f,
        Function::with_parameters(
            f.name().to_owned(),
            f.parameter_count(),
            f.operations().to_vec(),
        )
        .unwrap()
    );
    assert_eq!(execute(&f, 0), 47);
    assert_eq!(execute(&f, 1), 49);
    assert!(f.to_ir_text().starts_with("ciq-ir 2"));
    let reloaded = parse_ir(&f.to_ir_text()).unwrap();
    assert_eq!(f, reloaded);
    assert_eq!(
        f.lower().encode().unwrap(),
        reloaded.lower().encode().unwrap()
    );
    assert_eq!(
        f.lower().encode().unwrap(),
        ciq_assembly::Program::parse(&f.assembly())
            .unwrap()
            .encode()
            .unwrap()
    );
    let greater=compile("function choose(value) { if ((value + 2) > 3) { return value - 1; } else { return value + 1; } }").unwrap();
    for (argument, expected) in [(0, 1), (1, 2), (2, 1), (9, 8)] {
        assert_eq!(execute(&greater, argument), expected);
    }
}

#[test]
fn unsupported_conditions_and_boolean_uses_report_source_diagnostics() {
    for source in [
        "function f(x) { if (x) { return 1; } else { return 2; } }",
        "function f(x) { if (x == 0) { return 1; } return 2; }",
        "function f(x) { if (x == 0) return 1; else return 2; }",
        "function f(x) { if (x == 0) { var a = 1; return a; } else { return 2; } }",
        "function f(x) { if (x == 0) { return 1; } else { return 2; } return 3; }",
        "function f(x) { if (\"x\" == x) { return 1; } else { return 2; } }",
        "function f(x) { return (x == 0) + 1; }",
        "function f(x) { return x > 0; }",
        "function f(x) { var a = x == 0; return 1; }",
        "function f(x) { return Numbers.call(x == 0); }",
        "function f(x) { if (x > 0 > 1) { return 1; } else { return 2; } }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

fn cfg(tail: &str) -> String {
    format!(
        "ciq-ir 2\n.function f\n.block ^0\n.integer %0 0\n.integer %1 1\n.equal %2 %0 %1\n.branch %2 ^1 ^2\n{tail}\n.end\n"
    )
}

#[test]
fn control_flow_rejects_bad_edges_terminators_and_cross_block_values() {
    for tail in [
        ".block ^1\n.integer %3 1\n.return %3\n.block ^2\n.jump ^0",
        ".block ^1\n.jump ^3\n.block ^2\n.integer %4 2\n.return %4",
        ".block ^1\n.integer %3 1\n.block ^2\n.integer %4 2\n.return %4",
        ".block ^1\n.integer %3 1\n.return %3\n.integer %9 0\n.block ^2\n.integer %4 2\n.return %4",
        ".block ^1\n.integer %3 1\n.return %3\n.block ^1\n.integer %4 2\n.return %4",
        ".block ^1\n.integer %3 1\n.return %3\n.block ^2\n.return %0",
        ".block ^1\n.integer %3 1\n.return %3\n.block ^2\n.integer %4 2\n.return %4\n.block ^3\n.integer %5 3\n.return %5",
    ] {
        let text = cfg(tail);
        assert!(parse_ir(&text).is_err(), "{text}");
    }
    let mut missing_entry =
        cfg(".block ^1\n.integer %3 1\n.return %3\n.block ^2\n.integer %4 2\n.return %4");
    missing_entry = missing_entry.replacen(".block ^0\n", "", 1);
    assert!(parse_ir(&missing_entry).is_err());
}

#[test]
fn joins_require_definite_local_initialization_without_inventing_phi_values() {
    let common = "ciq-ir 2\n.function f\n.block ^0\n.integer %0 7\n.local $0 %0\n.integer %1 0\n.integer %2 1\n.greater %3 %1 %2\n.branch %3 ^1 ^2\n.block ^1\n.jump ^3\n.block ^2\n.jump ^3\n.block ^3\n.read %4 $0\n.return %4\n.end";
    let f = parse_ir(common).unwrap();
    assert_eq!(execute(&f, 0), 7);
    let partial = cfg(
        ".block ^1\n.integer %3 7\n.local $0 %3\n.jump ^3\n.block ^2\n.jump ^3\n.block ^3\n.read %4 $0\n.return %4",
    );
    assert!(
        parse_ir(&partial)
            .unwrap_err()
            .message
            .contains("local read")
    );
    // A local that only exists in the first textual arm is also unavailable in its sibling.
    let sibling = cfg(
        ".block ^1\n.integer %3 7\n.local $0 %3\n.integer %4 1\n.return %4\n.block ^2\n.read %5 $0\n.return %5",
    );
    assert!(parse_ir(&sibling).is_err());
}

#[test]
fn effects_remain_in_order_and_inside_the_selected_block() {
    let source = "function f(x) { System.println(x + 1); if (Numbers.left() == Numbers.right()) { return Numbers.yes(); } else { return Numbers.no(); } }";
    let f = compile(source).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let assembly = f.assembly();
    assert!(
        assembly.find("__ciq_member_left").unwrap() < assembly.find("__ciq_member_right").unwrap()
    );
    assert!(
        assembly.find("__ciq_block_f_1:\n").unwrap() < assembly.find("__ciq_member_yes").unwrap()
    );
    assert!(
        assembly.find("__ciq_block_f_2:\n").unwrap() < assembly.find("__ciq_member_no").unwrap()
    );
    let text = f.to_ir_text();
    let lines: Vec<_> = text.lines().collect();
    let left = lines
        .iter()
        .position(|l| l.contains("Numbers left"))
        .unwrap();
    let right = lines
        .iter()
        .position(|l| l.contains("Numbers right"))
        .unwrap();
    let mut swapped = lines.clone();
    swapped.swap(left, right);
    assert!(parse_ir(&swapped.join("\n")).is_err());
    assert!(parse_ir(&text.replacen("ciq-ir 2", "ciq-ir 1", 1)).is_err());
}
