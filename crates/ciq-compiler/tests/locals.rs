use ciq_compiler::{LocalId, Operation, ValueId, compile, parse_ir};
use ciq_mc::{Opcode, decode};
use std::collections::BTreeMap;

fn evaluate(bytes: &[u8]) -> i32 {
    // Restricted instruction check; this does not model Garmin's physical frame.
    let instructions = decode(bytes).unwrap();
    let first = &instructions[0].instruction;
    assert_eq!(first.opcode(), Opcode::Argcincsp);
    assert_eq!(first.operands()[0], 1);
    let count = first.operands()[1];
    let mut slots = BTreeMap::new();
    let mut stack = Vec::new();
    for (index, located) in instructions.iter().enumerate().skip(1) {
        let instruction = &located.instruction;
        match instruction.opcode() {
            Opcode::Ipushz => stack.push(0),
            Opcode::Ipush1 => stack.push(instruction.operands()[0] as i8 as i32),
            Opcode::Addv | Opcode::Subv => {
                let right = stack.pop().unwrap();
                let left = stack.pop().unwrap();
                stack.push(if instruction.opcode() == Opcode::Addv {
                    left + right
                } else {
                    left - right
                });
            }
            Opcode::Lputv => {
                let slot = instruction.operands()[0];
                assert!((1..=count).contains(&slot));
                assert!(slots.insert(slot, stack.pop().unwrap()).is_none());
                assert!(stack.is_empty(), "initializer leaves temporary values");
            }
            Opcode::Lgetv => stack.push(slots[&instruction.operands()[0]]),
            Opcode::Return => {
                assert_eq!(index + 1, instructions.len());
                assert_eq!(stack.len(), 1);
                return stack.pop().unwrap();
            }
            other => panic!("unexpected opcode {other:?}"),
        }
    }
    panic!("missing return")
}

fn check(body: &str, expected: i32) {
    let ir = compile(&format!("function probe() {{ {body} }}")).unwrap();
    let bytes = ir.lower().encode().unwrap();
    assert_eq!(evaluate(&bytes), expected);
    let resumed = parse_ir(&ir.to_ir_text()).unwrap();
    assert_eq!(resumed, ir);
    assert_eq!(resumed.lower().encode().unwrap(), bytes);
    assert_eq!(
        ciq_assembly::Program::parse(&ir.assembly())
            .unwrap()
            .encode()
            .unwrap(),
        bytes
    );
}

#[test]
fn locals_repeated_reads_and_initialization_order_survive_all_pipelines() {
    check("var x = 40 + 2; return x;", 42);
    check("var x = 2 - 5; return x + x;", -6);
    check("var x = 0; var y = x + 7; return y - x;", 7);
    check("var unused = 1; return 43;", 43);
    check(
        "var first = 40 + 2; var second = first - 5; return first + second - first;",
        37,
    );
    let bytes = compile("function probe() { var x = 40 + 2; return x; }")
        .unwrap()
        .lower()
        .encode()
        .unwrap();
    assert_eq!(
        bytes,
        [0x4a, 1, 1, 0x38, 40, 0x38, 2, 3, 0x13, 1, 0x12, 1, 0x16]
    );
}

#[test]
fn semantic_locals_have_distinct_identity_and_source_spans() {
    let source = "function probe() { var answer = 41; return answer + answer; }";
    let ir = compile(source).unwrap();
    assert_eq!(
        ir.operations()[1].operation,
        Operation::LocalInitialize {
            local: LocalId(0),
            value: ValueId(0)
        }
    );
    assert_eq!(
        ir.operations()[2].operation,
        Operation::LocalRead {
            result: ValueId(2),
            local: LocalId(0)
        }
    );
    assert_eq!(
        ir.operations()[3].operation,
        Operation::LocalRead {
            result: ValueId(3),
            local: LocalId(0)
        }
    );
    for (index, expected) in [(1, "var answer = 41;"), (2, "answer"), (3, "answer")] {
        let span = ir.operations()[index].span.unwrap();
        assert_eq!(&source[span.start..span.end], expected);
    }
}

#[test]
fn invalid_local_source_is_rejected_with_locations() {
    for body in [
        "var x; return x;",
        "var x = x; return x;",
        "var x = y; var y = 41; return x;",
        "var x = 1; var x = 2; return x;",
        "var return = 41; return 41;",
        "var var = 41; return 41;",
        "var x = y, y = 2; return x;",
        "var x = 1; return y;",
        "return 1; var x = 2;",
        "{ var x = 1; } return x;",
    ] {
        assert!(
            compile(&format!("function probe() {{ {body} }}")).is_err(),
            "{body}"
        );
    }
    let error = compile("function probe() {\n    var x = missing;\n    return x;\n}").unwrap_err();
    assert_eq!((error.line, error.column), (2, 13));
    assert!(error.message.contains("not yet initialized"));
    assert!(compile("function var() { return 1; }").is_err());
}

#[test]
fn declaration_lists_share_scope_order_and_lowering_with_individual_declarations() {
    let flat = compile("function probe(){var x=1;var y=x+2;var z=y+x;return z;}").unwrap();
    let grouped = compile("function probe(){var x=1,y=x+2,z=y+x;;;return z;}").unwrap();
    assert_eq!(flat.assembly(), grouped.assembly());
    assert_eq!(grouped, parse_ir(&grouped.to_ir_text()).unwrap());
    for body in [
        "var x=1,x=2;return x;",
        "var x=1,y=y;return x;",
        "var x=1,var y=2;return x;",
        "var x=1,;return x;",
        "var x=1,y;return y;",
        "{var x=1,y=x+1;}return y;",
    ] {
        assert!(
            compile(&format!("function probe(){{{body}}}")).is_err(),
            "{body}"
        );
    }
    let names = (0..256)
        .map(|i| format!("n{i}=0"))
        .collect::<Vec<_>>()
        .join(",");
    assert!(
        compile(&format!("function probe(){{var {names};return 0;}}"))
            .unwrap_err()
            .message
            .contains("255-local limit")
    );
}

#[test]
fn extra_semicolons_terminate_effects_without_creating_empty_statements() {
    for (flat, extra) in [
        ("{var x=1;}return 0;", "{var x=1;};;;return 0;"),
        (
            "var x=1;x+=2;x++;return x;",
            "var x=1;;;x+=2;;;x++;;;return x;",
        ),
        (
            "System.println(1);return 0;",
            "System.println(1);/*extra*/;;return 0;",
        ),
        (
            "var x=0;do{x++;}while(false);return x;",
            "var x=0;do{x++;}while(false);;;return x;",
        ),
    ] {
        let flat = compile(&format!("function probe(){{{flat}}}")).unwrap();
        let extra = compile(&format!("function probe(){{{extra}}}")).unwrap();
        assert_eq!(flat.assembly(), extra.assembly());
        assert_eq!(extra, parse_ir(&extra.to_ir_text()).unwrap());
    }
    for body in [
        ";return 0;",
        "if(true){;}return 0;",
        "if(true){};return 0;",
        "return 0;;",
        "for(;;){break;;}return 0;",
        "for(;;){continue;;}return 0;",
        "for(var i=0;;;i<2;i++){}return 0;",
    ] {
        assert!(
            compile(&format!("function probe(){{{body}}}")).is_err(),
            "{body}"
        );
    }
}

#[test]
fn local_text_and_typed_validation_reject_uninitialized_or_duplicate_locals() {
    for body in [
        ".read %0 $9\n.return %0",
        ".read %0 $9\n.integer %1 41\n.local $9 %1\n.return %0",
        ".integer %0 41\n.local $9 %0\n.integer %1 42\n.local $9 %1\n.read %2 $9\n.return %2",
        ".integer %0 41\n.local $9 %0\n.read %1 $8\n.return %1",
        ".integer %0 41\n.local $9 %8\n.return %0",
        ".integer %0 41\n.local %9 %0\n.read %1 $9\n.return %1",
        ".integer %0 41\n.local $4294967296 %0\n.read %1 $9\n.return %1",
    ] {
        assert!(
            parse_ir(&format!("ciq-ir\n.function probe\n{body}\n.end")).is_err(),
            "{body}"
        );
    }
    let constructed = ciq_compiler::Function::new(
        "probe".into(),
        vec![
            ciq_compiler::LocatedOperation {
                operation: Operation::LocalRead {
                    result: ValueId(0),
                    local: LocalId(1),
                },
                span: None,
            },
            ciq_compiler::LocatedOperation {
                operation: Operation::Return { value: ValueId(0) },
                span: None,
            },
        ],
    )
    .unwrap_err();
    assert_eq!(constructed.operation, Some(0));
    assert!(constructed.message.contains("earlier initialized"));
}

#[test]
fn semantic_local_ids_do_not_select_vm_slots() {
    let text = "ciq-ir\n.function probe\n.integer %10 40\n.local $4294967295 %10\n.read %20 $4294967295\n.integer %30 3\n.add %40 %20 %30\n.local $9 %40\n.read %50 $9\n.read %60 $4294967295\n.subtract %70 %50 %60\n.return %70\n.end";
    let ir = parse_ir(text).unwrap();
    let bytes = ir.lower().encode().unwrap();
    assert_eq!(evaluate(&bytes), 3);
    assert_eq!(parse_ir(&ir.to_ir_text()).unwrap(), ir);
    let slots: Vec<_> = decode(&bytes)
        .unwrap()
        .iter()
        .filter(|i| i.instruction.opcode() == Opcode::Lputv)
        .map(|i| i.instruction.operands()[0])
        .collect();
    assert_eq!(slots, [1, 2]);
}

#[test]
fn local_and_total_operation_limits_include_initializers_and_reads() {
    let declarations = (0..255)
        .map(|n| format!("var x{n} = 1; "))
        .collect::<String>();
    check(&format!("{declarations}return x254;"), 1);
    assert!(
        compile(&format!(
            "function probe() {{ {declarations}var x255 = 1; return x255; }}"
        ))
        .unwrap_err()
        .message
        .contains("255-local")
    );
    let initializers = (0..255)
        .map(|n| format!("var x{n} = 1 + 1; "))
        .collect::<String>();
    check(&format!("{initializers}return 1;"), 1); // 1020 + 2 operations
    assert!(
        compile(&format!(
            "function probe() {{ {initializers}return 1 + 1; }}"
        ))
        .is_ok()
    ); // exactly 1024
    assert!(
        compile(&format!(
            "function probe() {{ {initializers}return 1 + 1 + 1; }}"
        ))
        .unwrap_err()
        .message
        .contains("operation limit")
    );
    let body = (0..256)
        .map(|n| format!(".integer %{n} 1\n.local ${n} %{n}\n"))
        .collect::<String>();
    assert!(
        parse_ir(&format!(
            "ciq-ir\n.function probe\n{body}.read %300 $255\n.return %300\n.end"
        ))
        .unwrap_err()
        .message
        .contains("255-local")
    );
}
