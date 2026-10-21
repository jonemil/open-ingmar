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
            Opcode::Btpush => stack.push(1),
            Opcode::Bfpush => stack.push(0),
            // These condition specimens use invv only on checked Booleans.
            Opcode::Invv => {
                let value = stack.pop().unwrap();
                assert!(matches!(value, 0 | 1));
                stack.push(i32::from(value == 0));
            }
            Opcode::Ipush1 => stack.push(ins.operands()[0] as i8 as i32),
            Opcode::Lgetv => stack.push(locals[ins.operands()[0] as usize]),
            Opcode::Lputv => locals[ins.operands()[0] as usize] = stack.pop().unwrap(),
            Opcode::Addv | Opcode::Subv | Opcode::Eq | Opcode::Lt | Opcode::Gt => {
                let right = stack.pop().unwrap();
                let left = stack.pop().unwrap();
                stack.push(match ins.opcode() {
                    Opcode::Addv => left + right,
                    Opcode::Subv => left - right,
                    Opcode::Eq => i32::from(left == right),
                    Opcode::Lt => i32::from(left < right),
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
    assert!(f.to_ir_text().starts_with("ciq-ir"));
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
        "function f(x) { if (x == 0) return 1; else return 2; }",
        "function f(x) { if (x == 0) { return 1; } else { return 2; } return 3; }",
        "function f(x) { if (\"x\" > x) { return 1; } else { return 2; } }",
        "function f(x) { return (x == 0) + 1; }",
        "function f(x) { return x > 0; }",
        "function f(x) { if (x > 0 > 1) { return 1; } else { return 2; } }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn boolean_literals_locals_and_assignments_control_branches_and_loops() {
    for (literal, opcode, expected) in [("true", "btpush", 1), ("false", "bfpush", 2)] {
        let source =
            format!("function f() {{ if ({literal}) {{ return 1; }} else {{ return 2; }} }}");
        let function = compile(&source).unwrap();
        assert!(function.assembly().contains(opcode));
        assert_eq!(execute(&function, 0), expected);
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    }
    let function = compile("function f(x) { var again = x > 0; var count = 0; while (again) { count = count + 1; again = x > count; } if (again == false) { return count; } else { return 127; } }").unwrap();
    for input in [0, 1, 7, 40] {
        assert_eq!(execute(&function, input), input);
    }
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    for source in [
        "function f() { var flag = true; flag = 1; return 0; }",
        "function f() { var n = 1; n = false; return 0; }",
        "function f() { return true + 1; }",
        "function f() { return false; }",
        "function f() { if (true == 1) { return 1; } else { return 2; } }",
        "function f() { if (true > false) { return 1; } else { return 2; } }",
        "function f() { var true = 1; return 0; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn boolean_text_literals_have_strict_syntax_and_keep_semantic_types() {
    let function =
        compile("function f() { var flag = true; flag = false; System.println(flag); }").unwrap();
    let text = function.to_ir_text();
    assert!(text.contains(".boolean %0 true"));
    assert_eq!(function, parse_ir(&text).unwrap());
    for invalid in ["True", "1", "0", "null", "\"true\""] {
        assert!(parse_ir(&text.replace("%0 true", &format!("%0 {invalid}"))).is_err());
    }
    assert!(parse_ir(&text.replace(".boolean %0 true", ".integer %0 1")).is_err());
    // A comparison can be stored and later printed, with each read owning its
    // own semantic value; storing does not relax one-use or scope rules.
    let function = compile("function f(n) { var zero = n == 0; System.println(zero); if (zero) { System.println(true); } }").unwrap();
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
}

fn cfg(tail: &str) -> String {
    format!(
        "ciq-ir\n.function f\n.block ^0\n.integer %0 0\n.integer %1 1\n.equal %2 %0 %1\n.branch %2 ^1 ^2\n{tail}\n.end\n"
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
    let common = "ciq-ir\n.function f\n.block ^0\n.integer %0 7\n.local $0 %0\n.integer %1 0\n.integer %2 1\n.greater %3 %1 %2\n.branch %3 ^1 ^2\n.block ^1\n.jump ^3\n.block ^2\n.jump ^3\n.block ^3\n.read %4 $0\n.return %4\n.end";
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
        "function f(x) { if (x == 0) { return 1; var n = 2; } else { return 3; } }",
        "function f(x) { if (x == 0) { return 1; } else { return 2; } return 3; }",
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
    // String returns remain unsupported; Boolean locals may hold comparisons.
    assert!(
        compile("function f(x) { if (x == 0) { var n = \"str\"; return n; } else { return 1; } }")
            .is_err()
    );
    assert!(
        compile("function f(x) { if (x == 0) { var n = x > 1; return 1; } else { return 2; } }")
            .is_ok()
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
    assert!(f.to_ir_text().starts_with("ciq-ir"));
}

#[test]
fn source_assignments_reject_type_changes_and_escaping_names() {
    for source in [
        "function f(x) { x = \"bad\"; return x; }",
        "function f() { n = 1; var n = 2; return n; }",
        "function f(x) { if (x == 0) { var n = 1; } else { n = 2; } return 1; }",
        "function f(x) { if (x == 0) { var n = 1; } else {} n = 2; return 1; }",
        "function f() { var n = 1; n = \"string\"; return n; }",
        "function f() { var n = \"string\"; n = 1; return 1; }",
        "function f(x) { var n = 1; n = x > 0; return n; }",
        "function f() { var n = 1; return n = 2; }",
        "function f() { var n = 1; (n) = 2; return n; }",
        "function f() { var n = 1; n + 1 = 2; return n; }",
        "function f() { var n = 1; n += \"string\"; return n; }",
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
    let stale = "ciq-ir\n.function f\n.integer %0 1\n.local $0 %0\n.read %1 $0\n.integer %2 2\n.assign $0 %2\n.return %1\n.end";
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
        assert!(parse_ir(&format!("ciq-ir\n.function f\n{body}\n.end")).is_err());
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

#[test]
fn while_tests_each_iteration_and_preserves_body_local_initialization() {
    let f = compile("function f(x) { var n = x; var total = 0; while (n > 0) { var current = n; if (current == 2) { total = total + 5; } else { total = total + current; } n = n - 1; } return total; }").unwrap();
    for (argument, expected) in [(0, 0), (1, 1), (2, 6), (3, 9), (4, 13), (9, 48)] {
        assert_eq!(execute(&f, argument), expected);
    }
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    assert_eq!(
        f.lower().encode().unwrap(),
        ciq_assembly::Program::parse(&f.assembly())
            .unwrap()
            .encode()
            .unwrap()
    );
    assert_eq!(
        Function::with_parameters(
            f.name().into(),
            f.parameter_count(),
            f.operations().to_vec()
        )
        .unwrap(),
        f
    );
    let code = decode(&f.lower().encode().unwrap()).unwrap();
    assert!(code.iter().any(|i| {
        i.relative_target()
            .is_some_and(|target| target < i.offset as i64)
    }));
    assert_eq!(code[0].instruction.operands(), &[2, 3]);
    let nested = compile("function f(x) { var n = x; var total = 0; while (n > 0) { var inner = n; while (inner > 0) { total = total + 1; inner = inner - 1; } n = n - 1; } return total; }").unwrap();
    for (argument, expected) in [(0, 0), (1, 1), (3, 6)] {
        assert_eq!(execute(&nested, argument), expected);
    }
}

#[test]
fn while_early_returns_leave_the_zero_iteration_exit_available() {
    let f = compile("function f(x) { var n = x; while (n > 0) { if (n == 2) { return 42; } else { n = n - 1; } } return 7; }").unwrap();
    for (argument, expected) in [(0, 7), (1, 7), (2, 42), (3, 42)] {
        assert_eq!(execute(&f, argument), expected);
    }
    let f = compile("function f(x) { while (x > 0) { return 41; } return 43; }").unwrap();
    assert_eq!(execute(&f, 0), 43);
    assert_eq!(execute(&f, 1), 41);
    let f = compile("function f(x) { var n = x; if (n > 0) { while (n > 0) { n = n - 1; } } else { n = 3; } return n; }").unwrap();
    assert_eq!(execute(&f, 0), 3);
    assert_eq!(execute(&f, 4), 0);
    assert!(
        compile("function f(x) { while (x > 0) { return 41; } }")
            .unwrap_err()
            .message
            .contains("every path")
    );
}

#[test]
fn cyclic_ir_reachability_and_initialization_do_not_invent_first_iteration_values() {
    // Numeric IDs and declaration order intentionally differ from execution order.
    let text = "ciq-ir\n.function f parameters=1\n.block ^100\n.jump ^5\n.block ^1\n.integer %0 7\n.local $0 %0\n.jump ^5\n.block ^5\n.parameter %1 0\n.integer %2 0\n.greater %3 %1 %2\n.branch %3 ^1 ^50\n.block ^50\n.read %4 $0\n.return %4\n.end";
    // Changing the first edge to the body makes its initializer definite at exit.
    let mandatory_body = text.replacen(".jump ^5", ".jump ^1", 1);
    assert_eq!(execute(&parse_ir(&mandatory_body).unwrap(), 0), 7);

    assert!(
        parse_ir(text)
            .unwrap_err()
            .message
            .contains("initialized local")
    );
    let assign = text.replace(
        ".read %4 $0\n.return %4",
        ".integer %4 2\n.assign $0 %4\n.integer %5 3\n.return %5",
    );
    assert!(
        parse_ir(&assign)
            .unwrap_err()
            .message
            .contains("incoming path")
    );
    // A loop header cannot read a body-only initializer, even on the backedge.
    let header = text
        .replace(".parameter %1 0", ".read %1 $0")
        .replace(".read %4 $0", ".integer %4 1");
    assert!(parse_ir(&header).is_err());
    let disconnected = "ciq-ir\n.function f\n.block ^0\n.jump ^3\n.block ^1\n.jump ^2\n.block ^2\n.jump ^1\n.block ^3\n.integer %0 1\n.return %0\n.end";
    let function = parse_ir(disconnected).unwrap();
    assert_eq!(execute(&function, 0), 1);
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    // An initializer inside a cyclic block must still precede its first local read.
    let bad = "ciq-ir\n.function f parameters=1\n.block ^0\n.jump ^1\n.block ^1\n.read %0 $0\n.local $0 %0\n.parameter %1 0\n.integer %2 0\n.greater %3 %1 %2\n.branch %3 ^1 ^2\n.block ^2\n.integer %4 1\n.return %4\n.end";
    assert!(parse_ir(bad).is_err());
}

#[test]
fn while_scopes_syntax_and_combined_nesting_limits_are_enforced() {
    for source in [
        "function f(x) { while (x) {} return 1; }",
        "function f(x) { while (x > 0) x = 1; return 1; }",
        "function f(x) { while (x > 0) { var n = 1; } return n; }",
        "function f(x) { while (x > 0) { var n = 1; } n = 2; return 1; }",
        "function f(x) { var n = 1; while (x > 0) { var n = 2; } return n; }",
        "function f() { var while = 1; return while; }",
        "function while() { return 1; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let nest = |n| {
        format!(
            "function f(x) {{ {} {} return 1; }}",
            "while (x > 0) {".repeat(n),
            "}".repeat(n)
        )
    };
    assert!(compile(&nest(64)).is_ok());
    assert!(compile(&nest(65)).unwrap_err().message.contains("nesting"));
    let mixed = format!(
        "function f(x) {{ {}{}return 1; {}{}return 2; }}",
        "if (x == 0) {".repeat(32),
        "while (x > 0) {".repeat(33),
        "}".repeat(33),
        "} else { return 1; }".repeat(32)
    );
    assert!(compile(&mixed).unwrap_err().message.contains("nesting"));
}

#[test]
fn loop_effects_and_text_pipeline_remain_inside_test_body_and_exit_blocks() {
    let f = compile(include_str!("../../../examples/compiler-probe/loops.mc")).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    assert_eq!(
        f.lower(),
        ciq_assembly::Program::parse(&f.assembly()).unwrap()
    );
    let f = compile("function f() { var n = 3; while (Numbers.condition(n) > 0) { n = Numbers.next(n); System.println(n); } return Numbers.after(n); }").unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let assembly = f.assembly();
    assert!(
        assembly.find("__ciq_member_condition").unwrap()
            > assembly.find("__ciq_block_f_1:").unwrap()
    );
    assert!(
        assembly.find("__ciq_member_next").unwrap() > assembly.find("__ciq_block_f_2:").unwrap()
    );
    assert!(
        assembly.find("__ciq_member_after").unwrap() > assembly.find("__ciq_block_f_3:").unwrap()
    );
    let strings = compile("function f(x) { var text = \"first\"; var n = x; while (n > 0) { System.println(text); text = \"next\"; n = n - 1; } System.println(text); return 1; }").unwrap();
    assert_eq!(strings, parse_ir(&strings.to_ir_text()).unwrap());
    assert!(compile("function f() { while (0 == 0) {} return 1; }").is_ok());
}

#[test]
fn nested_loop_control_selects_innermost_targets_and_preserves_pipeline() {
    let source = include_str!("../../../examples/compiler-probe/loop-control.mc")
        .replace("    System.println(total);\n", "");
    let f = compile(&source).unwrap();
    for (argument, expected) in [(0, 44), (1, 46), (2, 48), (5, 54)] {
        assert_eq!(execute(&f, argument), expected);
    }
    let restored = compile("function f(x) { var n = x; while (n > 0) { while (n > 0) { break; } n = n - 1; continue; } return n; }").unwrap();
    for argument in 0..4 {
        assert_eq!(execute(&restored, argument), 0);
    }
    let zero = compile(&source.replace("value + 4", "value")).unwrap();
    for (argument, expected) in [(0, 40), (1, 42), (2, 42), (3, 42), (4, 44)] {
        assert_eq!(execute(&zero, argument), expected);
    }
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    assert_eq!(
        f.lower().encode().unwrap(),
        ciq_assembly::Program::parse(&f.assembly())
            .unwrap()
            .encode()
            .unwrap()
    );
}

#[test]
fn mixed_loop_terminators_omit_joins_without_hiding_loop_exit() {
    for (source, results) in [
        (
            "function f(x) { var n = x; while (n > 0) { n = n - 1; if (n > 1) { continue; } else { break; } } return n; }",
            [0, 0, 1, 1],
        ),
        (
            "function f(x) { var n = x; while (n > 0) { n = n - 1; if (n > 1) { continue; } else { return 7; } } return 9; }",
            [9, 7, 7, 7],
        ),
        (
            "function f(x) { while (x > 0) { if (x == 1) { break; } else { return 7; } } return 9; }",
            [9, 9, 7, 7],
        ),
    ] {
        let f = compile(source).unwrap();
        for (argument, expected) in results.into_iter().enumerate() {
            assert_eq!(execute(&f, argument as i32), expected);
        }
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
}

#[test]
fn optional_else_preserves_false_path_scope_and_return_requirements() {
    let f = compile(
        "function f(x) { var n = 3; if (x == 0) { return 7; } if (x > 1) { n = 9; } return n; }",
    )
    .unwrap();
    for (argument, expected) in [(0, 7), (1, 3), (2, 9)] {
        assert_eq!(execute(&f, argument), expected);
    }
    let f = compile(
        "function f(x) { if (x == 0) { if (x > 1) { return 1; } } else { return 2; } return 3; }",
    )
    .unwrap();
    assert_eq!(execute(&f, 0), 3);
    assert_eq!(execute(&f, 1), 2);
    for source in [
        "function f(x) { if (x == 0) { return 1; } }",
        "function f(x) { if (x == 0) { var n = 1; } return n; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn scoped_blocks_and_else_if_preserve_values_and_pipeline_round_trips() {
    for (source, outcomes) in [
        (
            "function f(x){var n=1;{var step=x;n+=step;}{var step=2;n+=step;}return n;}",
            [3, 4, 5],
        ),
        (
            "function f(x){{if(x==0){return 41;}else if(x==1){return 42;}else{return 43;}}}",
            [41, 42, 43],
        ),
        (
            "function f(x){var n=1;if(x==0){n=7;}else if(x==1){n=9;}return n;}",
            [7, 9, 1],
        ),
        (
            "function f(x){var n=0;for(var i=0;i<4;i++){{if(i==1){continue;}else if(i==3){break;}else{n+=i;}}}return n+x;}",
            [2, 3, 4],
        ),
        (
            "function f(x){var n=x;{} {while(n>0){{n-=1;}}}return n;}",
            [0, 0, 0],
        ),
    ] {
        let function = compile(source).unwrap();
        for (argument, expected) in outcomes.into_iter().enumerate() {
            assert_eq!(execute(&function, argument as i32), expected, "{source}");
        }
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
        assert_eq!(
            function.lower(),
            ciq_assembly::Program::parse(&function.assembly()).unwrap()
        );
    }
    let flat = compile("function f(){var n=41;return n;}").unwrap();
    let scoped = compile("function f(){{var n=41;return n;}}").unwrap();
    assert_eq!(flat.assembly(), scoped.assembly());

    let source = "function f(n as Toybox.Lang.Number or Null){{if(n==null){return 0;}else if(n>0){return n+1;}else{return n-1;}}}";
    let unit = ciq_compiler::compile_sources(&[source]).unwrap();
    assert_eq!(
        unit,
        ciq_compiler::parse_unit_ir(&unit.to_ir_text()).unwrap()
    );
}

#[test]
fn scoped_blocks_and_else_if_keep_scope_reachability_and_nesting_limits() {
    for source in [
        "function f(){{var n=1;}return n;}",
        "function f(x){{var x=1;}return 0;}",
        "function f(){var n=1;{var n=2;}return n;}",
        "function f(){{return 1;}return 2;}",
        "function f(){{for(;;){return 1;}}return 2;}",
        "function f(){{continue;}return 1;}",
        "function f(){{break;}return 1;}",
        "function f(x){if(x==0){return 1;}else if(x==1){return 2;}else{return 3;}return 4;}",
        "function f(x){if(x==0){var n=1;}else if(n==1){return 2;}return 3;}",
        "function f(x){if(x==0){return 1;}else if(x>0) return 2;return 3;}",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    for depth in [64, 65] {
        let blocks = format!(
            "function f(){{{}return 1;{}}}",
            "{".repeat(depth),
            "}".repeat(depth)
        );
        let chain = format!(
            "function f(x){{{}{{return 2;}}}}",
            "if(x==0){return 1;}else ".repeat(depth)
        );
        for source in [blocks, chain] {
            let result = compile(&source);
            assert_eq!(result.is_ok(), depth == 64, "{source}: {result:?}");
            if let Err(error) = result {
                assert!(error.message.contains("nesting"));
            }
        }
    }
    let empty_blocks = format!("function f(){{{}return 1;}}", "{}".repeat(1025));
    assert!(compile(&empty_blocks).unwrap_err().message.contains("1024"));
}

#[test]
fn control_flow_bodies_require_braces_as_in_the_sdk() {
    for source in [
        "function f(x){if(x==0)return 1;else return 2;}",
        "function f(){var i=0;while(i<2)i++;return i;}",
        "function f(){var i=0;for(i=0;i<2;i++);return i;}",
        "function f(){var i=0;do i++;while(i<2);return i;}",
        "function f(x){if(x==0)if(x==1){return 1;}else{return 2;}return 3;}",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn switch_dispatch_and_control_targets_survive_saved_stages() {
    for source in [
        "function f(x){switch(x){case 0:return 41;case 1:return 42;default:return 43;}}",
        "function f(x){var n=0;switch(x){case 0:n+=1;case 1:n+=2;break;default:n=9;}return n;}",
        "function f(x){switch(x){default:return 13;case 0:return 41;case 1:return 42;}}",
        "function f(x){switch(x){case 0:case 1:return 7;default:return 9;}}",
        "function f(x){var n=9;switch(x){case 0:n=1;break;case 1:n=2;break;}return n;}",
        "function f(x){switch(x){case 0:return 1;case 0:return 2;default:return 3;}}",
        "function f(x){var n=0;switch(x){case 0:n+=1;default:n+=3;case 1:n+=2;}return n;}",
        "function f(x){switch(x){case x+1:return 1;case x:return 2;default:return 3;}}",
        "function f(x){var n=0;for(var i=0;i<4;i++){switch(i){case 1:continue;case 3:break;default:n+=i;}n+=10;}return n+x;}",
        "function f(x){var n=0;switch(x){case 0:for(var i=0;i<3;i++){n++;break;}return n;default:return 9;}}",
        "function f(x){var n=0;switch(x){case 0:switch(x){case 0:break;}n++;break;default:n=9;}return n;}",
        "function f(x){switch(x){case 0:{var n=1;return n;}default:{var n=2;return n;}}}",
    ] {
        let function = compile(source).unwrap();
        assert!(function.to_ir_text().contains("equals"));
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
        assert_eq!(
            function.lower(),
            ciq_assembly::Program::parse(&function.assembly()).unwrap()
        );
    }
}

#[test]
fn switches_retain_type_guards_and_unit_obligations() {
    for source in [
        "function f(n as Toybox.Lang.Object){switch(n){case instanceof Toybox.Lang.Number:return n+1;case instanceof Toybox.Lang.String:return n.length();default:return 0;}}",
        "function f(n as Toybox.Lang.Number or Null){if(n==null){return 0;}switch(n){case 4:return 5;default:return n+1;}}",
        "class Token{function initialize(){} function value(){return 1;}} function f(n as Toybox.Lang.Object){switch(n){case instanceof Token:return n.value();default:return 0;}}",
        "module M {function selector(){return 1;} function test(n){return n;} function f(){switch(M.selector()){case M.test(0):return 1;case M.test(1):return 2;default:return 3;}}}",
        "function f(x){switch(x){default:System.println(x);}}",
    ] {
        let unit = ciq_compiler::compile_sources(&[source]).unwrap();
        assert_eq!(
            unit,
            ciq_compiler::parse_unit_ir(&unit.to_ir_text()).unwrap()
        );
        assert_eq!(
            unit.lower(),
            ciq_assembly::Program::parse(&unit.lower().to_text()).unwrap()
        );
    }
}

#[test]
fn switch_scope_initialization_labels_and_nesting_are_checked() {
    for source in [
        "function f(n as Toybox.Lang.Number or Null){switch(n){case null:return 0;default:return n+1;}}",
        "function f(x){switch(x){}return 0;}",
        "function f(x){switch(x){return 1;}return 0;}",
        "function f(x){switch(x){default:return 1;default:return 2;}}",
        "function f(x){switch(x){case 0:return 1;}}",
        "function f(x){switch(x){case 0:continue;default:break;}return 0;}",
        "function f(x){switch(x){case 0:break;return 1;}return 0;}",
        "function f(x){switch(x){default:return 1;}return 2;}",
        "function f(x){switch(x){case 0:var n=1;break;case 1:var n=2;break;}return 0;}",
        "function f(x){switch(x){case 0:var n=1;case 1:return n;default:return 0;}}",
        "function f(x){switch(x){case 0:var n=1;break;}return n;}",
        "function f(){switch(missing){default:return 0;}}",
        "function f(x){switch(x){case missing:return 1;default:return 0;}}",
        "function f(x){switch(x){case instanceof Unknown:return 1;default:return 0;}}",
        "function f(x){switch(x){case 0 return 1;default:return 0;}}",
        "function f(n as Toybox.Lang.Object){switch(n){case instanceof Toybox.Lang.String:System.println(n);case instanceof Toybox.Lang.Number:return n+1;default:return 0;}}",
    ] {
        assert!(
            ciq_compiler::compile_sources(&[source]).is_err(),
            "{source}"
        );
    }
    for depth in [64, 65] {
        let source = format!(
            "function f(x){{{}return 1;{}}}",
            "switch(x){default:".repeat(depth),
            "}".repeat(depth)
        );
        let result = compile(&source);
        assert_eq!(result.is_ok(), depth == 64, "{result:?}");
        if let Err(error) = result {
            assert!(error.message.contains("nesting"));
        }
    }
    let source = format!(
        "function f(x){{switch(x){{{}default:return 0;}}}}",
        "case 0:".repeat(1025)
    );
    assert!(compile(&source).unwrap_err().message.contains("1024"));
}

#[test]
fn for_header_lists_execute_left_to_right_and_preserve_loop_control() {
    for (source, expected) in [
        (
            "function f(){var sum=0;for(var i=0,j=i+1;i<3;i++,j+=i){sum+=j;}return sum;}",
            7,
        ),
        (
            "function f(){var sum=0;var i=0;var j=0;for(i=1,j=i+1;i<3;i++,j+=i){sum+=j;}return sum;}",
            6,
        ),
        (
            "function f(){var sum=0;for(var i=0,j=0;i<4;i++,j+=i){if(i==1){continue;}if(i==3){break;}sum+=j;}return sum;}",
            3,
        ),
        (
            "function f(){var i=0;var j=0;for(i++,++j;i<3;i++,j+=i){}return j;}",
            6,
        ),
        (
            "function f(){var i=0;var j=0;for(;i<3;i++,j+=i){break;}return j;}",
            0,
        ),
        (
            "function f(){for(var i=0,j=i+1;i<2;i++,j++){}var i=41;var j=1;return i+j;}",
            42,
        ),
    ] {
        let function = compile(source).unwrap();
        assert_eq!(execute(&function, 0), expected, "{source}");
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
        assert_eq!(
            function.lower(),
            ciq_assembly::Program::parse(&function.assembly()).unwrap()
        );
    }
}

#[test]
fn loop_control_rejects_outside_targets_reserved_names_and_unreachable_code() {
    for keyword in ["break", "continue"] {
        for source in [
            format!("function f() {{ {keyword}; return 1; }}"),
            format!("function f(x) {{ if (x > 0) {{ {keyword}; }} return 1; }}"),
            format!("function f(x) {{ while (x > 0) {{ break; }} {keyword}; return 1; }}"),
            format!("function f() {{ var {keyword} = 1; return 1; }}"),
            format!("function {keyword}() {{ return 1; }}"),
            format!("function f(x) {{ while (x > 0) {{ {keyword} }} return 1; }}"),
        ] {
            assert!(compile(&source).is_err(), "{source}");
        }
        let source =
            format!("function f(x) {{ while (x > 0) {{ {keyword}; return 1; }} return 2; }}");
        assert!(
            compile(&source)
                .unwrap_err()
                .message
                .contains("unreachable")
        );
    }
    let source = "function f(x) { while (x > 0) { if (x == 1) { break; } else { continue; } return 1; } return 2; }";
    assert!(compile(source).unwrap_err().message.contains("unreachable"));
}

#[test]
fn short_circuit_conditions_follow_boolean_truth_tables_and_precedence() {
    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                for (expression, expected) in [
                    (format!("{a} && {b}"), a && b),
                    (format!("{a} || {b}"), a || b),
                    (format!("{a} || {b} && {c}"), a || b && c),
                    (format!("({a} || {b}) && {c}"), (a || b) && c),
                    (format!("!({a} && {b} || {c})"), !(a && b || c)),
                ] {
                    let f = compile(&format!(
                        "function f() {{ if ({expression}) {{ return 1; }} else {{ return 0; }} }}"
                    ))
                    .unwrap();
                    assert_eq!(execute(&f, 0), i32::from(expected), "{expression}");
                    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
                }
            }
        }
    }
    let f = compile("function f(n) { var count = 0; while (n > count && 3 > count) { count = count + 1; } return count; }").unwrap();
    for n in [0, 1, 2, 3, 4] {
        assert_eq!(execute(&f, n), n.min(3));
    }
}

#[test]
fn logical_values_materialize_boolean_results_and_retest_assignments() {
    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                for (expression, expected) in [
                    (format!("{a} && {b}"), a && b),
                    (format!("{a} || {b} && {c}"), a || b && c),
                    (format!("!({a} || {b}) == {c}"), !(a || b) == c),
                    (
                        format!("({a} && {b}) == ({b} || {c})"),
                        (a && b) == (b || c),
                    ),
                ] {
                    let f = compile(&format!(
                        "function f() as Toybox.Lang.Boolean {{ return {expression}; }}"
                    ))
                    .unwrap();
                    assert_eq!(execute(&f, 0), i32::from(expected), "{expression}");
                    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
                }
            }
        }
    }
    let f = compile("function f(n) { var flag = n > 0 && true; var count = 0; while (flag) { count = count + 1; flag = n > count && 3 > count; } return count; }").unwrap();
    for n in [0, 1, 2, 3, 4] {
        assert_eq!(execute(&f, n), n.min(3));
    }
}

#[test]
fn for_and_post_test_loops_have_distinct_latches_and_correct_scope() {
    for (source, expected) in [
        (
            "function f() { var sum=0; for(var i=0; 6>i; i=i+1) { if(i==1){continue;} if(i==5){break;} sum=sum+i; } return sum; }",
            9,
        ),
        (
            "function f() { var n=0; do { n=n+1; if(3>n){continue;} break; } while(true); return n; }",
            3,
        ),
        (
            "function f() { var n=0; do {n=n+1;} while(false); return n; }",
            1,
        ),
        ("function f() { var n=0; for(;2>n;){n=n+1;}return n; }", 2),
        ("function f() { for(;;){break;}return 41; }", 41),
        ("function f() { for(;;){return 41;} }", 41),
        ("function f() { do {return 41;} while(false); }", 41),
        (
            "function f() { for(var i=0;2>i;i=i+1){break;}var i=41;return i; }",
            41,
        ),
        (
            "function f() { var n=0; for(var i=0;3>i;i=i+1){var j=0;do{j=j+1;if(j==2){continue;}n=n+1;}while(3>j);}return n; }",
            6,
        ),
    ] {
        let function = compile(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(execute(&function, 0), expected, "{source}");
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
        assert_eq!(
            function.lower(),
            ciq_assembly::Program::parse(&function.assembly()).unwrap()
        );
    }
}

#[test]
fn loop_header_scope_and_unexecuted_clauses_still_receive_diagnostics() {
    for source in [
        "function f() {for(var i=0;3>i;i=i+1){}return i;}",
        "function f() {for(var i=0;3>i;i=i+1){var i=2;}return 0;}",
        "function f() {var i=0;for(var i=0;3>i;i=i+1){}return 0;}",
        "function f() {for(var i=0;3>i;missing++){break;}return 0;}",
        "function f() {for(var i=0;3>i;i=false){break;}return 0;}",
        "function f() {do{break;}while(missing);return 0;}",
        "function f() {do{break;}while(1);return 0;}",
        "function f() {do{var n=0;}while(n==0);return 0;}",
        "function f() {for(var i=0;3>i;var j=0){}return 0;}",
        "function f() {do{}while(false)return 0;}",
        "function f() {for(;;)break;return 0;}",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let source = "function f() {for(;;){continue;}}";
    let function = compile(source).unwrap();
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    let function = compile("function f() as Toybox.Lang.Number {for(;;){continue;}}").unwrap();
    assert_eq!(function.return_type(), ciq_compiler::ReturnType::Integer);
    assert!(function.to_ir_text().contains("result=integer"));
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
}

#[test]
fn disconnected_blocks_stay_checked_without_changing_executable_proofs() {
    let valid = "ciq-ir\n.function f\n.block ^0\n.integer %0 41\n.local $0 %0\n.jump ^2\n.block ^1\n.read %1 $0\n.discard %1\n.jump ^2\n.block ^2\n.read %2 $0\n.return %2\n.end\n";
    let function = parse_ir(valid).unwrap();
    assert_eq!(execute(&function, 0), 41);
    assert!(parse_ir(&valid.replace(".read %1 $0", ".read %1 $9")).is_err());
    assert!(
        parse_ir(&valid.replace(
            ".read %1 $0\n.discard %1",
            ".boolean %1 false\n.assign $0 %1"
        ))
        .is_err()
    );
    let void_tail = "ciq-ir\n.function f\n.block ^0\n.integer %0 41\n.return %0\n.block ^1\n.return-void\n.end\n";
    let function = parse_ir(void_tail).unwrap();
    assert_eq!(execute(&function, 0), 41);
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
}
