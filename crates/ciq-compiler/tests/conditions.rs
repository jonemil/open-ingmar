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
                let condition = stack.pop().unwrap();
                assert!(stack.is_empty(), "branch carries expression values");
                if condition == 0 {
                    pc = located.relative_target().unwrap() as usize;
                }
            }
            Opcode::Goto => {
                assert!(stack.is_empty(), "jump carries expression values");
                pc = located.relative_target().unwrap() as usize;
            }
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

#[test]
fn nested_arms_scoped_locals_early_returns_and_joins_execute_correctly() {
    let source = "function f(x) {
        var common = 40;
        if (x == 0) {
            var amount = 3;
            if (amount > 2) {
                var result = common + amount;
                return result;
            } else { return common; }
        } else { var amount = 5; }
        var result = common + 7;
        return result;
    }";
    let f = compile(source).unwrap();
    assert_eq!(execute(&f, 0), 43);
    assert_eq!(execute(&f, 1), 47);
    assert_eq!(
        execute(
            &compile(&source.replace("amount > 2", "amount > 3")).unwrap(),
            0
        ),
        40
    );
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    assert_eq!(
        f.lower().encode().unwrap(),
        ciq_assembly::Program::parse(&f.assembly())
            .unwrap()
            .encode()
            .unwrap()
    );
    assert!(
        f.operations()
            .iter()
            .any(|o| matches!(o.operation, ciq_compiler::Operation::Jump { .. }))
    );
    // Both arms continue and repeated conditions read common locals at joins.
    let f = compile("function f(x) { var common = 7; if (x == 0) { var n = common + 1; } else { var n = common + 2; } if (x > 0) { return common + 1; } else { return common; } }").unwrap();
    assert_eq!(execute(&f, 0), 7);
    assert_eq!(execute(&f, 1), 8);
    // Returning only from the false arm still leaves a valid true-arm join.
    let f = compile("function f(x) { if (x == 0) { var n = 1; } else { return 2; } return 3; }")
        .unwrap();
    assert_eq!(execute(&f, 0), 3);
    assert_eq!(execute(&f, 1), 2);
}

#[test]
fn source_arm_scopes_missing_returns_and_unreachable_statements_are_rejected() {
    for source in [
        "function f(x) { if (x == 0) { var n = 1; } else { var n = 2; } return n; }",
        "function f(x) { if (x == 0) { var n = 1; return n; } else { return n; } }",
        "function f(x) { var n = 1; if (x == 0) { var n = 2; return n; } else { return n; } }",
        "function f(x) { if (x == 0) { var x = 1; return x; } else { return 2; } }",
        "function f(x) { if (x == 0) { var n = 1; var n = 2; return n; } else { return 2; } }",
        "function f(x) { if (x == 0) { return 1; } else {} }",
        "function f(x) { if (x == 0) {} else {} }",
        "function f(x) { if (x == 0) { return 1; var n = 2; } else { return 3; } }",
        "function f(x) { if (x == 0) { return 1; } else { return 2; } return 3; }",
        "function f(x) { if (x == 0) {} else { if (x > 1) {} else {} } }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let source = "function f(x) { if (x == 0) { var n = 1; } else {}\n return n; }";
    let error = compile(source).unwrap_err();
    assert_eq!(&source[error.offset..error.offset + 1], "n");
    assert_eq!(error.line, 2);
    let error = compile("function f(x) { if (x == 0) { return 1; } else {} }").unwrap_err();
    assert!(error.message.contains("every path"));
}

#[test]
fn nested_effects_text_resumption_and_arm_local_ids_preserve_order() {
    let source = include_str!("../../../examples/compiler-probe/nested-conditions.mc");
    let f = compile(source).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let assembly = f.assembly();
    assert_eq!(f.lower(), ciq_assembly::Program::parse(&assembly).unwrap());
    let locals: Vec<_> = f
        .operations()
        .iter()
        .filter_map(|o| match o.operation {
            ciq_compiler::Operation::LocalInitialize { local, .. } => Some(local.0),
            _ => None,
        })
        .collect();
    assert_eq!(locals, [0, 1, 2, 3, 4]);
    // An effectful initializer in a continuing arm remains ahead of its print.
    let f = compile("function f(x) { if (x == 0) { var n = Numbers.first(); System.println(n); } else { var n = Numbers.second(); System.println(n); } return Numbers.after(); }").unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let assembly = f.assembly();
    assert!(
        assembly.find("__ciq_member_first").unwrap() < assembly.find("__ciq_block_f_2:").unwrap()
    );
    assert!(
        assembly.find("__ciq_member_second").unwrap() < assembly.find("__ciq_block_f_3:").unwrap()
    );
    assert!(
        assembly.find("__ciq_member_after").unwrap() > assembly.find("__ciq_block_f_3:").unwrap()
    );
    // Boolean and string type restrictions still apply to branch-local uses.
    assert!(
        compile("function f(x) { if (x == 0) { var n = \"str\"; return n; } else { return 1; } }")
            .is_err()
    );
    assert!(
        compile("function f(x) { if (x == 0) { var n = x > 1; return 1; } else { return 2; } }")
            .is_err()
    );
}

#[test]
fn condition_nesting_and_global_frame_operation_limits_are_bounded() {
    let nested = |count: usize| {
        format!(
            "function f(x) {{ {}return 1; {} }}",
            "if (x == 0) {".repeat(count),
            "} else { return 2; }".repeat(count)
        )
    };
    assert!(compile(&nested(64)).is_ok());
    assert!(
        compile(&nested(65))
            .unwrap_err()
            .message
            .contains("nesting")
    );
    let locals = (0..128)
        .map(|i| format!("var n{i} = 1;"))
        .collect::<String>();
    let error = compile(&format!(
        "function f(x) {{ if (x == 0) {{ {locals} }} else {{ {locals} }} return 1; }}"
    ))
    .unwrap_err();
    assert!(error.message.contains("slot limit"));
    // Printing has one argument definition and one sink, with no call result.
    let prints = "System.println(1);".repeat(511);
    let f = compile(&format!("function f() {{ {prints}return 1; }}")).unwrap();
    assert_eq!(f.operations().len(), 1024);
    assert!(
        compile(&format!(
            "function f() {{ {prints}System.println(1); return 1; }}"
        ))
        .is_err()
    );
    // Many empty joins consume real block and terminator operations, not only expressions.
    let body = "if (x == 0) {} else {}".repeat(160);
    assert!(
        compile(&format!("function f(x) {{ {body}return 1; }}"))
            .unwrap_err()
            .message
            .contains("operation limit")
    );
}

#[test]
fn reassignment_keeps_local_slots_and_reads_the_selected_branch_value() {
    for (source, outcomes) in [
        (
            "function f(x) { var n = 40; var old = n; n = n + 1; n = n + 2; return n + old; }",
            [83, 83],
        ),
        (
            "function f(x) { var n = 40; if (x == 0) { n = n + 3; } else { n = n + 7; } n = n + 1; return n; }",
            [44, 48],
        ),
        (
            "function f(x) { var n = 40; if (x == 0) { n = 41; } else {} return n; }",
            [41, 40],
        ),
        (
            "function f(x) { var n = 40; if (x == 0) { n = 41; if (n > 40) { n = n + 2; } else {} } else { return n; } return n; }",
            [43, 40],
        ),
    ] {
        let f = compile(source).unwrap();
        for (argument, expected) in outcomes.into_iter().enumerate() {
            assert_eq!(execute(&f, argument as i32), expected);
        }
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        assert_eq!(
            f.lower().encode().unwrap(),
            ciq_assembly::Program::parse(&f.assembly())
                .unwrap()
                .encode()
                .unwrap()
        );
        let prologue = &decode(&f.lower().encode().unwrap()).unwrap()[0].instruction;
        assert_eq!(
            prologue.operands()[1],
            if source.contains("var old") { 2 } else { 1 }
        );
    }
    let source = "function f() { var n = 1; n = n + 2; return n; }";
    let f = compile(source).unwrap();
    let assigned = f
        .operations()
        .iter()
        .find(|o| matches!(o.operation, ciq_compiler::Operation::LocalAssign { .. }))
        .unwrap();
    let span = assigned.span.unwrap();
    assert_eq!(&source[span.start..span.end], "n = n + 2;");
    assert!(f.to_ir_text().starts_with("ciq-ir 2"));
}

#[test]
fn source_assignments_reject_type_changes_parameters_and_escaping_names() {
    for source in [
        "function f(x) { x = 1; return x; }",
        "function f() { n = 1; var n = 2; return n; }",
        "function f(x) { if (x == 0) { var n = 1; } else { n = 2; } return 1; }",
        "function f(x) { if (x == 0) { var n = 1; } else {} n = 2; return 1; }",
        "function f() { var n = 1; n = \"string\"; return n; }",
        "function f() { var n = \"string\"; n = 1; return 1; }",
        "function f(x) { var n = 1; n = x > 0; return n; }",
        "function f() { var n = 1; return n = 2; }",
        "function f() { var n = 1; (n) = 2; return n; }",
        "function f() { var n = 1; n + 1 = 2; return n; }",
        "function f() { var n = 1; n += 1; return n; }",
        "function f() { var n = 1; n++; return n; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let source = "function f() { var n = 1;\n n = \"string\"; return n; }";
    let error = compile(source).unwrap_err();
    assert_eq!((error.line, error.column), (2, 2));
    assert!(error.message.contains("preserve"));
}

#[test]
fn assignment_ir_requires_definite_initialization_and_rejects_stale_read_schedules() {
    let stale = "ciq-ir 2\n.function f\n.integer %0 1\n.local $0 %0\n.read %1 $0\n.integer %2 2\n.assign $0 %2\n.return %1\n.end";
    assert!(
        parse_ir(stale)
            .unwrap_err()
            .message
            .contains("evaluation order")
    );
    // The same stale read must also be rejected through direct typed construction.
    let ordered = stale
        .replace(".read %1 $0\n", "")
        .replace(".return %1", ".read %1 $0\n.return %1");
    let f = parse_ir(&ordered).unwrap();
    assert_eq!(execute(&f, 0), 2);
    let mut operations = f.operations().to_vec();
    let read = operations.remove(4);
    operations.insert(2, read);
    assert!(
        Function::new("f".into(), operations)
            .unwrap_err()
            .message
            .contains("evaluation order")
    );
    for body in [
        ".integer %0 1\n.assign $0 %0\n.integer %1 2\n.return %1",
        ".integer %0 1\n.local $0 %0\n.string %1 \"str\"\n.assign $0 %1\n.integer %2 2\n.return %2",
        ".string %0 \"str\"\n.local $0 %0\n.integer %1 1\n.assign $0 %1\n.return %1",
    ] {
        assert!(parse_ir(&format!("ciq-ir 2\n.function f\n{body}\n.end")).is_err());
    }
    let partial = cfg(
        ".block ^1\n.integer %3 1\n.local $0 %3\n.jump ^3\n.block ^2\n.jump ^3\n.block ^3\n.integer %4 2\n.assign $0 %4\n.integer %5 3\n.return %5",
    );
    assert!(
        parse_ir(&partial)
            .unwrap_err()
            .message
            .contains("incoming path")
    );
    assert!(parse_ir(&ordered.replacen("ciq-ir 2", "ciq-ir 1", 1)).is_err());
    // Aliases need not be small slot indexes; assignment reuses the initializer slot.
    let f = parse_ir(&ordered.replace("$0", "$4294967295")).unwrap();
    assert_eq!(execute(&f, 0), 2);
}

#[test]
fn assignment_effect_order_and_operation_limits_include_each_write() {
    let f = compile("function f() { var n = Numbers.first(); n = Numbers.second(n); System.println(n); return n; }").unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let assembly = f.assembly();
    assert!(
        assembly.find("__ciq_member_first").unwrap()
            < assembly.find("__ciq_member_second").unwrap()
    );
    let body = "n = 1;".repeat(510);
    let f = compile(&format!("function f() {{ var n = 1; {body}return n; }}")).unwrap();
    assert_eq!(f.operations().len(), 1024);
    assert_eq!(execute(&f, 0), 1);
    assert!(
        compile(&format!(
            "function f() {{ var n = 1; {body}n = 1; return n; }}"
        ))
        .unwrap_err()
        .message
        .contains("operation limit")
    );
}
