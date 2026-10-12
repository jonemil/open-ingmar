use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources_with_api, compile_with_api, parse_ir, parse_ir_with_api,
    parse_unit_ir_with_api,
};

#[test]
fn char_literals_preserve_vm_encoding_and_complete_pipelines() {
    for (value, bytes) in [
        (0x41, [0x34, 0, 0, 0, 0x41]),
        (0x732b, [0x34, 0, 0, 0x73, 0x2b]),
        (0x1f600, [0x34, 0, 1, 0xf6, 0]),
        (u32::MAX, [0x34, 0xff, 0xff, 0xff, 0xff]),
    ] {
        let assembly = ciq_assembly::Program::parse(&format!("cpush 0x{value:08x}")).unwrap();
        let encoded = assembly.encode().unwrap();
        assert_eq!(encoded, bytes);
        let instructions = ciq_mc::decode(&encoded).unwrap();
        assert_eq!(instructions[0].instruction.opcode(), ciq_mc::Opcode::Cpush);
        assert!(ciq_mc::decode(&encoded[..4]).is_err());
        let text = format!("ciq-ir\n.function f\n.char %0 0x{value:08x}\n.return %0\n.end\n");
        let f = parse_ir(&text).unwrap();
        assert_eq!(f.return_type(), ciq_compiler::ReturnType::Char);
        assert_eq!(f.to_ir_text(), text);
        assert!(f.lower().to_text().contains("cpush"));
    }
    let context = ApiContext::new(ApiLevel(5, 0, 0), ApiLevel(5, 0, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[
            include_str!("../../../examples/compiler-char-literals/app.mc"),
            include_str!("../../../examples/compiler-char-literals/chars.mc"),
        ],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [29; 16],
        entry: "start".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
}

#[test]
fn source_char_tokens_preserve_values_spans_and_reject_unsupported_forms() {
    for (literal, value) in [
        ("'A'", 65),
        ("'é'", 233),
        ("'猫'", 29483),
        (r"'\n'", 10),
        (r"'\r'", 13),
        (r"'\t'", 9),
        (r"'\''", 39),
        (r"'\\'", 92),
        ("'\"'", 34),
    ] {
        let source = format!("function f() as Toybox.Lang.Char {{ return {literal}; }}");
        let f = compile(&source).unwrap();
        let op = &f.operations()[0];
        assert!(
            matches!(op.operation, ciq_compiler::Operation::Char { value: actual, .. } if actual == value)
        );
        let span = op.span.unwrap();
        assert_eq!(&source[span.start..span.end], literal);
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    for literal in [
        "''",
        "'ab'",
        "'😀'",
        "'a",
        "'\n'",
        r"'\0'",
        r"'\u0041'",
        r"'\x41'",
        "'é'",
    ] {
        assert!(
            compile(&format!(
                "function f() as Toybox.Lang.Char {{ return {literal}; }}"
            ))
            .is_err(),
            "{literal:?}"
        );
    }
    for bits in ["65", "0x41", "0x000000041", "0x000000gg", "-1"] {
        assert!(
            parse_ir(&format!(
                "ciq-ir\n.function f\n.char %0 {bits}\n.return %0\n.end\n"
            ))
            .is_err()
        );
    }
    assert!(compile("function f() { return 'A'; }").is_err());
    assert!(compile("function f() { return 'A' + 1; }").is_err());
    let f = compile("function f() { return ('A'.toNumber() + 1).abs(); }").unwrap();
    assert!(parse_ir(&f.to_ir_text().replace("toNumber", "toUpper")).is_err());
    assert!(
        compile("function f() as Toybox.Lang.Char { var c = 'A'; c = 65; return c; }").is_err()
    );
}

#[test]
fn char_literal_api_introductions_agree_in_source_and_ir() {
    let source = "function f() as Toybox.Lang.Char { return 'A'; }";
    let text = compile(source).unwrap().to_ir_text();
    for (target, minimum, expected) in [
        (
            ApiLevel(1, 2, 9),
            ApiLevel(1, 0, 0),
            "unavailable on target",
        ),
        (
            ApiLevel(1, 3, 0),
            ApiLevel(1, 2, 9),
            "application minimum API",
        ),
    ] {
        let context = ApiContext::new(target, minimum).unwrap();
        let error = compile_with_api(source, context).unwrap_err();
        assert!(error.message.contains(expected));
        assert_eq!(
            error.message,
            parse_ir_with_api(&text, context).unwrap_err().message
        );
    }
    let context = ApiContext::new(ApiLevel(1, 3, 0), ApiLevel(1, 3, 0)).unwrap();
    assert_eq!(
        compile_with_api(source, context).unwrap(),
        parse_ir_with_api(&text, context).unwrap()
    );
}
