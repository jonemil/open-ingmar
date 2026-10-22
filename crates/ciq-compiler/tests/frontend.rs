use ciq_compiler::{Operation, ValueId, compile_function};
use ciq_mc::{Opcode, decode};

#[test]
fn semantic_ir_preserves_value_identity_and_source_spans() {
    let source = "// comment\nfunction probe() as Toybox.Lang.Number {\n    return 41;\n}\n";
    let function = compile_function(source).unwrap();
    assert_eq!(function.name(), "probe");
    let operations = function.operations();
    assert_eq!(
        operations[0].operation,
        Operation::Integer {
            result: ValueId(0),
            value: 41
        }
    );
    assert_eq!(
        operations[1].operation,
        Operation::Return { value: ValueId(0) }
    );
    assert_eq!(
        &source[operations[0].span.unwrap().start..operations[0].span.unwrap().end],
        "41"
    );
    assert_eq!(
        &source[operations[1].span.unwrap().start..operations[1].span.unwrap().end],
        "return 41;"
    );
}

#[test]
fn every_supported_literal_assembles_and_returns_its_value() {
    // Interpret only the supported constant-return sequence, independently of
    // the frontend IR. This is an encoding check, not a full VM/runtime oracle.
    for expected in 0..=127 {
        let function = compile_function(&format!(
            "function probe() as Toybox.Lang.Number {{ return {expected}; }}"
        ))
        .unwrap();
        let bytes = ciq_asm::assemble(&function.assembly()).unwrap();
        let mut result = None;
        let mut returned = false;
        let instructions = decode(&bytes).unwrap();
        assert_eq!(instructions[0].instruction.opcode(), Opcode::Argc);
        assert_eq!(instructions[0].instruction.operands(), &[1]);
        for instruction in &instructions[1..] {
            match instruction.instruction.opcode() {
                Opcode::Ipushz => result = Some(0),
                Opcode::Ipush1 => result = Some(instruction.instruction.operands()[0] as i8 as i32),
                Opcode::Return => {
                    assert_eq!(result, Some(expected));
                    returned = true;
                }
                other => panic!("unexpected instruction {other:?}"),
            }
        }
        assert!(returned);
        assert_eq!(
            ciq_asm::code_labels(&function.assembly()).unwrap()["probe"],
            0
        );
    }
}

#[test]
fn unsupported_programs_are_rejected_completely() {
    for source in [
        "",
        "function probe(x as Number) as Toybox.Lang.Number { return 41; }",
        "function probe() as Toybox.Lang.Number { return 2147483648; }",
        "function probe() as Toybox.Lang.Number { return 999999999999999999999999; }",
        "function probe() as Toybox.Lang.Number { return -2147483649; }",
        "function probe() as Toybox.Lang.Number { return 0x100000000; }",
        "function probe() as Toybox.Lang.Number { return 40 ** 1; }",
        "function probe() as Toybox.Lang.Number { return true; }",
        "function probe() as Toybox.Lang.Number { return 41; return 42; }",
        "function probe() as Toybox.Lang.Number { return 41; } function other() as Toybox.Lang.Number { return 42; }",
        "function return() as Toybox.Lang.Number { return 41; }",
        "module Left { function probe() as Toybox.Lang.Number { return 41; } }",
        "function probe() { return 41;",
        "function probe() { /* unterminated",
    ] {
        assert!(
            compile_function(source).is_err(),
            "accepted unsupported input {source:?}"
        );
    }
}

#[test]
fn comments_and_unicode_diagnostics_keep_correct_source_locations() {
    let function = compile_function(
        "/* café */ function /* name */ probe(// args\n) as Toybox.Lang.Number { return /* value */ 41; } // eof",
    )
    .unwrap();
    assert_eq!(function.name(), "probe");
    let source = "/* café */\nfunction probe() as Toybox.Lang.Number {\n  return 2147483648;\n}";
    let error = compile_function(source).unwrap_err();
    assert_eq!((error.line, error.column), (3, 10));
    assert_eq!(&source[error.offset..error.offset + 10], "2147483648");
    let error = compile_function("/* é */ function probe() as Toybox.Lang.Number { return 💡; }")
        .unwrap_err();
    assert_eq!((error.line, error.column), (1, 57));
    assert!(error.message.contains("unsupported character"));
}

#[test]
fn trailing_container_commas_preserve_values_effects_and_saved_stages() {
    for source in [
        "function f() as Toybox.Lang.Number { return [1,2,][1]; }",
        "function f() as Toybox.Lang.Number { return [M.first(),M.second(),][1]; }",
        "function f(choose as Toybox.Lang.Boolean) as Toybox.Lang.Number { return [choose ? 1 : 2,3,][0]; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Char> { return ['a','b',]; }",
        "function f() as Toybox.Lang.ByteArray { return [1,255,]b; }",
        "function f() as Toybox.Lang.ByteArray { return [M.first(),M.second(),]b; }",
        "function f() as Toybox.Lang.Dictionary { return {\"one\"=>1,\"two\"=>2,}; }",
        "function f() as Toybox.Lang.Dictionary { return {\"one\"=>M.first(),\"two\"=>M.second(),}; }",
    ] {
        let function = compile_function(source).unwrap();
        let ordinary = compile_function(&source.replace(",]", "]").replace(",}", "}")).unwrap();
        assert_eq!(
            function
                .operations()
                .iter()
                .map(|op| &op.operation)
                .collect::<Vec<_>>(),
            ordinary
                .operations()
                .iter()
                .map(|op| &op.operation)
                .collect::<Vec<_>>(),
            "{source}"
        );
        assert_eq!(
            function,
            ciq_compiler::parse_function_ir(&function.to_ir_text()).unwrap()
        );
        let assembly = function.lower();
        assert_eq!(
            assembly,
            ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
        );
    }
    compile_function("function f() as Toybox.Lang.Number { return [1, /* end */ ][0]; }").unwrap();
    compile_function("function f() as Toybox.Lang.Dictionary { return {\"a\" => 1, // end\n}; }")
        .unwrap();
    for source in [
        "function f() as Toybox.Lang.Number { return [,]; }",
        "function f() as Toybox.Lang.Number { return [1,,]; }",
        "function f() as Toybox.Lang.Number { return [,]b; }",
        "function f() as Toybox.Lang.Number { return [1,,2]b; }",
        "function f() as Toybox.Lang.Dictionary { return {,}; }",
        "function f() as Toybox.Lang.Dictionary { return {\"a\"=>1,,}; }",
        "function f() as Toybox.Lang.Dictionary { return {\"a\"=>,}; }",
        "function f() as Toybox.Lang.Number { return M.value(1,); }",
        "function f(n as Toybox.Lang.Number,) as Toybox.Lang.Number { return n; }",
        "function f() as Toybox.Lang.Number { var a=1,; return a; }",
        "function f() as Toybox.Lang.Number { for(var a=0,;a<1;a++){ return a; } return 1; }",
        "function f() as Toybox.Lang.Number { for(var a=0;a<1;a++,){ return a; } return 1; }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
}
