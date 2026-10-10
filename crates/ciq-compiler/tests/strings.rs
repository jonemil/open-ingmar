use ciq_assembly::{Item, Program};
use ciq_compiler::{compile, parse_ir, string_bindings, symbol_bindings};
use ciq_mc::{Opcode, decode};
use ciq_prg::data::Source;

fn symbols() -> Program {
    Program::parse(".symbol system \"Toybox_System\" id=0x00800005\n.symbol print \"println\" id=0x00800067\n.symbol user \"globals/Numbers\" id=0x02\n.symbol answer \"answer\" id=0x03\n").unwrap()
}

#[test]
fn strings_prints_and_interleaved_locals_survive_native_and_text_pipelines() {
    let source = r#"function probe() {
        System.println("héllo # span=1:2\n\"quoted\"\\");
        var text = "héllo # span=1:2\n\"quoted\"\\";
        System.println(text);
        var number = Numbers.answer();
        System.println(number + 2);
        System.println(text);
        return number;
    }"#;
    let function = compile(source).unwrap();
    assert_eq!(parse_ir(&function.to_ir_text()).unwrap(), function);
    let program = function.lower();
    let text = Program::parse(&program.to_text()).unwrap();
    assert_eq!(program.items(), text.items());
    let literals: Vec<_> = program
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::String(literal) => Some(literal),
            _ => None,
        })
        .collect();
    assert_eq!(literals.len(), 1);
    assert_eq!(literals[0].value, "héllo # span=1:2\n\"quoted\"\\");
    assert_eq!(literals[0].at, None);
    let data = Source::parse(&format!(
        ".bytes at=0x00000000 aabb\n{}",
        literals[0].to_text()
    ))
    .unwrap();
    let strings = string_bindings(&program, &data).unwrap();
    let symbols = symbol_bindings(&program, &symbols()).unwrap();
    let bytes = program.encode_with_references(&symbols, &strings).unwrap();
    assert_eq!(
        bytes,
        text.encode_with_references(&symbols, &strings).unwrap()
    );
    let instructions = decode(&bytes).unwrap();
    let mut news = 0;
    let mut prints = 0;
    for (index, located) in instructions.iter().enumerate() {
        let instruction = &located.instruction;
        if instruction.opcode() == Opcode::News {
            assert_eq!(instruction.operands(), &[0, 0, 0, 2]);
            news += 1;
        }
        if instruction.opcode() == Opcode::Invokem {
            assert_eq!(instruction.operands(), &[2]);
            assert_eq!(instructions[index + 1].instruction.opcode(), Opcode::Popv);
            prints += 1;
        }
    }
    assert_eq!((news, prints), (2, 4));
    assert!(program.encode().is_err());
    // DATA encoding verifies byte length rather than character count.
    let (payload, _) = data.encode().unwrap();
    assert_eq!(
        ciq_prg::data::string_at(&payload, 2).unwrap().0,
        literals[0].value
    );
}

#[test]
fn literal_binding_requires_unique_content_and_valid_placement() {
    let program = compile("function probe() { System.println(\"hello\"); return 1; }")
        .unwrap()
        .lower();
    for data in [
        "",
        ".string other \"different\"",
        ".string one \"hello\"\n.string two \"hello\"",
        ".string __ciq_string_probe_0 \"different\"\n.string two \"hello\"",
    ] {
        assert!(
            string_bindings(&program, &Source::parse(data).unwrap()).is_err(),
            "{data}"
        );
    }
    let pinned =
        Program::parse(".string greeting \"hello\" at=0x00000000\nnews &greeting").unwrap();
    let data = Source::parse(".string actual \"hello\"").unwrap();
    let bindings = string_bindings(&pinned, &data).unwrap();
    assert_eq!(
        pinned.encode().unwrap(),
        pinned
            .encode_with_references(&Default::default(), &bindings)
            .unwrap()
    );
    assert!(string_bindings(&pinned, &Source::parse(".string wrong \"oops\"").unwrap()).is_err());
    let program = Program::parse("news &actual").unwrap();
    assert!(
        program
            .encode_with_references(
                &Default::default(),
                &string_bindings(&program, &data).unwrap()
            )
            .is_ok()
    );
}

#[test]
fn source_and_ir_reject_unsupported_string_types_and_print_shapes() {
    for source in [
        "function probe() { return \"x\"; }",
        "function probe() { var x = \"x\"; return x + 1; }",
        "function probe() { return Numbers.answer(\"x\"); }",
        "function probe() { return System.println(1); }",
        "function probe() { var x = System.println(1); return x; }",
        "function probe() { System.println(); return 1; }",
        "function probe() { System.println(1, 2); return 1; }",
        "function probe(System) { System.println(1); return 1; }",
        "function probe() { var System = 1; System.println(1); return 1; }",
        "function probe() { System.print(1); return 1; }",
        "function probe() { System.println(\"bad\\q\"); return 1; }",
        "function probe() { System.println(\"raw\nnewline\"); return 1; }",
        "function probe() { System.println(\"unterminated); return 1; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    for body in [
        ".string %0 \"x\"\n.return %0",
        ".string %0 \"x\"\n.local $0 %0\n.read %1 $0\n.integer %2 1\n.add %3 %1 %2\n.return %3",
        ".string %0 \"x\"\n.call %1 Numbers answer %0\n.return %1",
        ".integer %0 1\n.integer %1 2\n.println %0\n.return %1",
        ".string %0 unquoted\n.println %0\n.integer %1 1\n.return %1",
        ".println %9\n.integer %0 1\n.return %0",
        ".integer %0, 1\n.return %0",
    ] {
        assert!(
            parse_ir(&format!("ciq-ir 1\n.function probe\n{body}\n.end")).is_err(),
            "{body}"
        );
    }
    let error = compile("function probe() { return \"x\"; }").unwrap_err();
    assert_eq!(error.column, 20);
    assert!(error.message.contains("integer"));
}

#[test]
fn utf8_literal_size_and_string_escape_limits_are_checked() {
    let largest = "a".repeat(65535);
    assert!(
        compile(&format!(
            "function probe() {{ System.println(\"{largest}\"); return 0; }}"
        ))
        .is_ok()
    );
    for oversized in ["a".repeat(65536), "é".repeat(32768)] {
        assert!(
            compile(&format!(
                "function probe() {{ System.println(\"{oversized}\"); return 0; }}"
            ))
            .is_err()
        );
    }
    let function =
        compile(r#"function probe() { System.println("\t\r\n\"\\"); return 0; }"#).unwrap();
    assert_eq!(parse_ir(&function.to_ir_text()).unwrap(), function);
    // IR uses assembly escapes, independently of the source escape subset.
    assert!(parse_ir("ciq-ir 1\n.function probe\n.string %0 \"\\u{1f642} #\" # comment\n.println %0\n.integer %1 0\n.return %1\n.end").is_ok());
}

#[test]
fn assigned_strings_preserve_types_bindings_slots_and_pipeline_parity() {
    let function = compile(include_str!(
        "../../../examples/compiler-probe/assignments.mc"
    ))
    .unwrap();
    let program = function.lower();
    let data =
        Source::parse(".string initial \"start\"\n.string zero \"zero\"\n.string one \"one\"")
            .unwrap();
    let strings = string_bindings(&program, &data).unwrap();
    let symbols = symbol_bindings(&program, &symbols()).unwrap();
    let bytes = program.encode_with_references(&symbols, &strings).unwrap();
    assert_eq!(
        bytes,
        parse_ir(&function.to_ir_text())
            .unwrap()
            .lower()
            .encode_with_references(&symbols, &strings)
            .unwrap()
    );
    assert_eq!(
        bytes,
        Program::parse(&program.to_text())
            .unwrap()
            .encode_with_references(&symbols, &strings)
            .unwrap()
    );
    let instructions = decode(&bytes).unwrap();
    assert_eq!(instructions[0].instruction.operands(), &[2, 2]);
    let stores: Vec<_> = instructions
        .iter()
        .filter(|i| i.instruction.opcode() == Opcode::Lputv)
        .map(|i| i.instruction.operands()[0])
        .collect();
    assert_eq!(stores, [2, 3, 2, 3, 2, 3, 2]);
    let offsets: Vec<_> = instructions
        .iter()
        .filter(|i| i.instruction.opcode() == Opcode::News)
        .map(|i| {
            i.instruction
                .operands()
                .iter()
                .fold(0u32, |v, b| (v << 8) | *b as u32)
        })
        .collect();
    assert_eq!(offsets, [0, 9, 17]);
}
