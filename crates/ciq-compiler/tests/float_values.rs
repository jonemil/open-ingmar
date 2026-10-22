use ciq_compiler::{
    Operation, ReturnType, ValueId,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_function_with_api, compile_sources, parse_function_ir,
    parse_function_ir_with_api, parse_unit_ir,
};

#[test]
fn floating_bits_survive_every_text_and_codec_boundary() {
    for (tag, return_type, width, values) in [
        (
            "float",
            ReturnType::Float,
            4,
            vec![
                0u64, 0x80000000, 0x3fa00000, 0x7f800000, 0xff800000, 0x7fc12345, 1,
            ],
        ),
        (
            "double",
            ReturnType::Double,
            8,
            vec![
                0,
                0x8000000000000000,
                0x3ff4000000000000,
                0x7ff0000000000000,
                0xfff0000000000000,
                0x7ff8000000001234,
                1,
            ],
        ),
    ] {
        for bits in values {
            let text = format!(
                "ciq-ir\n.function f\n.{tag} %0 0x{bits:0digits$x}\n.return %0\n.end\n",
                digits = width * 2
            );
            let f = parse_function_ir(&text).unwrap();
            assert_eq!(f.return_type(), return_type);
            assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
            let asm = f.lower();
            assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
            let encoded = asm.encode().unwrap();
            let decoded = ciq_mc::decode(&encoded).unwrap();
            let instruction = decoded
                .iter()
                .find(|i| {
                    matches!(
                        i.instruction.opcode(),
                        ciq_mc::Opcode::Fpush
                            | ciq_mc::Opcode::Fpushz
                            | ciq_mc::Opcode::Dpush
                            | ciq_mc::Opcode::Dpushz
                    )
                })
                .unwrap();
            if bits == 0 {
                assert!(instruction.instruction.operands().is_empty());
            } else {
                assert_eq!(
                    instruction.instruction.operands(),
                    &bits.to_be_bytes()[8 - width..]
                );
            }
            for bad in [
                "0",
                "-1",
                "0x1",
                "0xgggggggg",
                "1.25",
                "NaN",
                "0x00000000000000000",
            ] {
                assert!(
                    parse_function_ir(
                        &text.replace(&format!("0x{bits:0digits$x}", digits = width * 2), bad)
                    )
                    .is_err(),
                    "{tag} {bad}"
                );
            }
        }
    }
    let f = ciq_compiler::Function::new(
        "f".into(),
        vec![
            ciq_compiler::LocatedOperation {
                operation: Operation::Float {
                    result: ValueId(0),
                    bits: 0x80000000,
                },
                span: None,
            },
            ciq_compiler::LocatedOperation {
                operation: Operation::Return { value: ValueId(0) },
                span: None,
            },
        ],
    )
    .unwrap();
    assert!(f.to_ir_text().contains(".float %0 0x80000000"));
}

#[test]
fn source_rounding_literal_syntax_and_promotion_are_explicit() {
    for (literal, ty, bits) in [
        ("1.25", "Float", ".float %0 0x3fa00000"),
        ("1.25f", "Float", ".float %0 0x3fa00000"),
        ("1f", "Float", ".float %0 0x3f800000"),
        (".5", "Float", ".float %0 0x3f000000"),
        (".5f", "Float", ".float %0 0x3f000000"),
        ("-.5f", "Float", ".float %0 0xbf000000"),
        ("-0f", "Float", ".float %0 0x80000000"),
        ("1e2f", "Float", ".float %0 0x42c80000"),
        ("NaN", "Float", ".float %0 0x7fc00000"),
        ("-NaN", "Float", ".float %0 0x7fc00000"),
        (".5d", "Double", ".double %0 0x3fe0000000000000"),
        ("-0.0", "Float", ".float %0 0x80000000"),
        ("16777217.0", "Float", ".float %0 0x4b800000"),
        ("1.25e2", "Float", ".float %0 0x42fa0000"),
        ("1e2", "Float", ".float %0 0x42c80000"),
        ("1.25d", "Double", ".double %0 0x3ff4000000000000"),
        ("-0.0d", "Double", ".double %0 0x8000000000000000"),
    ] {
        let f = compile_function(&format!(
            "function f() as Toybox.Lang.{ty} {{ return {literal}; }}"
        ))
        .unwrap();
        assert!(f.to_ir_text().contains(bits));
    }
    for (expression, ty) in [
        ("1.25 + 2", "Float"),
        ("2 + 1.25", "Float"),
        ("1.25 + 2l", "Double"),
        ("2l + 1.25", "Double"),
        ("1.25d + 2.0", "Double"),
        ("2.0 + 1.25d", "Double"),
        ("1.25d + 2l", "Double"),
        ("2l + 1.25d", "Double"),
        ("+1.25", "Float"),
        ("-(1.25d)", "Double"),
    ] {
        let f = compile_function(&format!(
            "function f() as Toybox.Lang.{ty} {{ return {expression}; }}"
        ))
        .unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    for source in [
        "function f() as Toybox.Lang.Double { return 1.25D; }",
        "function f() as Toybox.Lang.Number { return 1.25; }",
        "function f() as Toybox.Lang.Float { return 1; }",
        "function f() as Toybox.Lang.Float { return 1.25d; }",
        "function f() as Toybox.Lang.Float { return 1.25 + 2l; }",
        "function f() as Toybox.Lang.Float { return 1e100; }",
        "function f() as Toybox.Lang.Float { return 1.25F; }",
        "function f() as Toybox.Lang.Double { return 1.25D; }",
        "function f() as Toybox.Lang.Float { return 01f; }",
        "function f() as Toybox.Lang.Float { return 01.5; }",
        "function f() as Toybox.Lang.Double { return 01d; }",
        "function f() as Toybox.Lang.Double { return 1e1000d; }",
        "function f() as Toybox.Lang.Float { return 1.2e+; }",
        "function f() as Toybox.Lang.Number { return 1.25 % 2; }",
        "function f() as Toybox.Lang.Number { return 1.25 & 2; }",
        "function f() as Toybox.Lang.Number { return 1.25 << 2; }",
        "function f() as Toybox.Lang.Number { return !1.25; }",
        "function f() as Toybox.Lang.Number { return ~1.25d; }",
        "function f() as Toybox.Lang.Number { return 1.25 + true; }",
        "function f() as Toybox.Lang.Float { var x = 1.25; x += 2l; return x; }",
        "function f() as Toybox.Lang.Float { return true ? 1.0 : 1; }",
        "function f() as Void { var xs = [1.25, 1]; }",
        "function f() as Void { var xs = {1 => 1.25, 2 => 1.25d}; }",
    ] {
        assert!(compile_function(source).is_err(), "accepted {source}");
    }
}

#[test]
fn grouped_application_preserves_native_and_text_pipelines() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-float-values/app.mc"
    )])
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [91; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
    assert_eq!(
        unit.lower(),
        ciq_assembly::Program::parse(&unit.lower().to_text()).unwrap()
    );
    let text = unit.to_ir_text();
    let edited = text.replace(
        "doubleParse result=nullable-double",
        "doubleParse result=double",
    );
    assert_ne!(text, edited);
    assert!(parse_unit_ir(&edited).is_err());
}

#[test]
fn native_profiles_and_nullable_proofs_are_rechecked() {
    for ty in ["Float", "Double"] {
        let literal = if ty == "Float" { "1.25" } else { "1.25d" };
        for method in [
            "abs()",
            "toFloat()",
            "toDouble()",
            "toNumber()",
            "toLong()",
            "toString()",
            "format(\"%.2f\")",
            "hashCode()",
            "weak()",
            "equals(null)",
            "compareTo(1)",
        ] {
            let f = compile_function(&format!(
                "function f() as Void {{ System.println({literal}.{method}); }}"
            ))
            .unwrap();
            assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        }
        for expression in [
            format!(
                "function f(x as Toybox.Lang.{ty} or Null) as Toybox.Lang.{ty} {{ if (x == null) {{ return {literal}; }} return x.abs(); }}"
            ),
            format!("function f() as Toybox.Lang.{ty} or Null {{ return \"1.25\".to{ty}(); }}"),
            format!(
                "function f() as Toybox.Lang.{ty} {{ var x = \"1.25\".to{ty}(); while (x == null) {{ x = \"2.5\".to{ty}(); }} return x; }}"
            ),
            format!(
                "function f() as Toybox.Lang.{ty} or Null {{ return true ? {literal} : null; }}"
            ),
        ] {
            let f = compile_function(&expression).unwrap();
            assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        }
        for body in [
            "return x.abs();",
            "return x + 1;",
            "if (x == null) { return ZERO; } x = null; return x;",
            "var copy = x; if (x == null) { return ZERO; } return copy;",
            "while (x == null) { break; } return x;",
        ] {
            assert!(
                compile_function(&format!(
                    "function f(x as Toybox.Lang.{ty} or Null) as Toybox.Lang.{ty} {{ {} }}",
                    body.replace("ZERO", literal)
                ))
                .is_err()
            );
        }
        for method in ["abs(1)", "format(1)", "toNumber(1)", "compareTo(null)"] {
            assert!(
                compile_function(&format!(
                    "function f() as Void {{ System.println({literal}.{method}); }}"
                ))
                .is_err()
            );
        }
    }
}

#[test]
fn api_boundaries_apply_to_source_and_saved_text() {
    for (expression, level, before) in [
        ("1.25.toDouble()", ApiLevel(1, 0, 0), None),
        ("1.25d.toFloat()", ApiLevel(1, 0, 0), None),
        ("1.toFloat()", ApiLevel(1, 0, 0), None),
        ("1l.toDouble()", ApiLevel(1, 0, 0), None),
        ("1.25.equals(null)", ApiLevel(1, 0, 0), None),
        (
            "1.25d.equals(null)",
            ApiLevel(1, 3, 0),
            Some(ApiLevel(1, 2, 9)),
        ),
        ("1.25.weak()", ApiLevel(1, 2, 0), Some(ApiLevel(1, 1, 9))),
        (
            "1.25d.compareTo(1.0)",
            ApiLevel(5, 0, 0),
            Some(ApiLevel(4, 9, 9)),
        ),
    ] {
        let source = format!("function f() as Void {{ System.println({expression}); }}");
        let context = ApiContext::new(level, level).unwrap();
        let f = compile_function_with_api(&source, context).unwrap();
        if let Some(before) = before {
            for c in [
                ApiContext::new(before, before).unwrap(),
                ApiContext::new(level, before).unwrap(),
            ] {
                assert!(compile_function_with_api(&source, c).is_err());
                assert!(parse_function_ir_with_api(&f.to_ir_text(), c).is_err());
            }
        }
    }
    for (ty, level, before) in [
        ("Float", ApiLevel(1, 0, 0), None),
        ("Double", ApiLevel(3, 1, 0), Some(ApiLevel(3, 0, 9))),
    ] {
        let source =
            format!("function f() as Toybox.Lang.{ty} or Null {{ return \"1.25\".to{ty}(); }}");
        let f = compile_function_with_api(&source, ApiContext::new(level, level).unwrap()).unwrap();
        if let Some(before) = before {
            let c = ApiContext::new(before, before).unwrap();
            assert!(compile_function_with_api(&source, c).is_err());
            assert!(parse_function_ir_with_api(&f.to_ir_text(), c).is_err());
        }
    }
}
