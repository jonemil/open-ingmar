use ciq_compiler::{
    ComparisonOp, Function, Operation,
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, parse_ir, parse_unit_ir,
};

#[test]
fn comparisons_have_direct_encodings_and_preserve_complete_pipelines() {
    for (operator, byte, directive) in [
        ("<", 0x1b, "less"),
        ("<=", 0x1c, "less-equal"),
        (">=", 0x1e, "greater-equal"),
        ("!=", 0x1f, "not-equal"),
    ] {
        let function = compile(&format!(
            "function f(a,b) as Toybox.Lang.Boolean {{ return a {operator} b; }}"
        ))
        .unwrap();
        assert_eq!(
            function.lower().encode().unwrap(),
            [0x35, 3, 0x12, 1, 0x12, 2, byte, 0x16]
        );
        assert!(function.to_ir_text().contains(&format!(".{directive} ")));
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    }
    let unit = compile_sources(&[
        include_str!("../../../examples/compiler-number-comparisons/app.mc"),
        include_str!("../../../examples/compiler-number-comparisons/comparisons.mc"),
    ])
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [56; 16],
        entry: "MinimalConsoleApp.onStart".into(),
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
fn inequality_guards_refine_only_the_correct_live_storage() {
    for body in [
        "if (n != null) { return n + 1; } return 0;",
        "if (null != n) { return n + 1; } return 0;",
        "if (!(n != null)) { return 0; } return n + 1;",
        "if (!!(n != null)) { return n + 1; } return 0;",
        "if (n != null && n <= 17) { return n + 1; } return 0;",
        "if (n != null || true) { if (n != null) { return n + 1; } } return 0;",
        "while (n != null) { return n + 1; } return 0;",
    ] {
        let function = compile(&format!(
            "function f() {{ var n = \"17\".toNumber(); {body} }}"
        ))
        .unwrap();
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    }
    for body in [
        "if (n != null) { return 0; } return n + 1;",
        "if (n != null) { n = null; return n + 1; } return 0;",
        "if (n != null) { return n + 1; } else { return n + 1; }",
        "if (n != null || true) { return n + 1; } return 0;",
        "if (n != null) {} return n + 1;",
        "var flag = n != null; if (flag) { return n + 1; } return 0;",
        "while (n != null) { break; } return n + 1;",
        "if (n != null) { n = \"bad\".toNumber(); return n + 1; } return 0;",
    ] {
        assert!(
            compile(&format!(
                "function f() {{ var n = \"17\".toNumber(); {body} }}"
            ))
            .is_err(),
            "{body}"
        );
    }
    let source =
        "function f() { var n = \"17\".toNumber(); if (n != null) { return n + 1; } return 0; }";
    let function = compile(source).unwrap();
    let mut operations = function.operations().to_vec();
    let branch = operations
        .iter_mut()
        .find(|op| matches!(op.operation, Operation::Branch { .. }))
        .unwrap();
    let Operation::Branch {
        on_true, on_false, ..
    } = &mut branch.operation
    else {
        unreachable!()
    };
    std::mem::swap(on_true, on_false);
    assert!(Function::new("f".into(), operations).is_err());
    let changed = function.to_ir_text().replace(".not-equal ", ".equal ");
    assert!(parse_ir(&changed).is_err());
    let stale = "ciq-ir\n.function f\n.block ^0\n.string %0 \"17\"\n.call-native %1 %0 toNumber\n.local $0 %1\n.read %2 $0\n.null %3\n.not-equal %4 %2 %3\n.null %5\n.assign $0 %5\n.branch %4 ^1 ^2\n.block ^1\n.read %6 $0\n.integer %7 1\n.add %8 %6 %7\n.return %8\n.block ^2\n.integer %9 0\n.return %9\n.end\n";
    assert!(parse_ir(stale).is_err());
    let parameter = "ciq-ir\n.function f parameters=1 parameter-types=nullable-integer\n.block ^0\n.parameter %0 0\n.null %1\n.not-equal %2 %0 %1\n.branch %2 ^1 ^2\n.block ^1\n.parameter %3 0\n.return %3\n.block ^2\n.integer %4 0\n.return %4\n.end\n";
    let function = parse_ir(parameter).unwrap();
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    assert!(parse_ir(&parameter.replace(".branch %2 ^1 ^2", ".branch %2 ^2 ^1")).is_err());
    let function = compile("function f() { var text = \"abc\".substring(null, 2); if (text != null) { return text.length(); } return 0; }").unwrap();
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
}

#[test]
fn comparisons_reject_unproven_or_incompatible_operands_and_chains() {
    for operator in ["<", "<=", ">=", "!="] {
        for value in ["[1]", "new WatchUi.View()"] {
            assert!(
                compile(&format!(
                    "function f() as Toybox.Lang.Boolean {{ return 1 {operator} {value}; }}"
                ))
                .is_err()
            );
        }
    }
    for operator in ["<", "<=", ">="] {
        for value in ["true", "null", "\"1\""] {
            assert!(
                compile(&format!(
                    "function f() as Toybox.Lang.Boolean {{ return {value} {operator} {value}; }}"
                ))
                .is_err()
            );
        }
        assert!(
            compile(&format!(
                "function f() as Toybox.Lang.Boolean {{ return \"17\".toNumber() {operator} 18; }}"
            ))
            .is_err()
        );
    }
    for expression in [
        "1 < 2 < 3",
        "1 <= 2 >= 3",
        "1 != 2 != 3",
        "1 !== 2",
        "1 < = 2",
        "1 <=",
        "1 !=",
    ] {
        assert!(
            compile(&format!(
                "function f() as Toybox.Lang.Boolean {{ return {expression}; }}"
            ))
            .is_err()
        );
    }
    for directive in ["less", "less-equal", "greater-equal", "not-equal"] {
        assert!(
            parse_ir(&format!(
                "ciq-ir\n.function f\n.integer %0 1\n.{directive} %1 %0\n.return %1\n.end\n"
            ))
            .is_err()
        );
    }
    let function =
        compile("function f() as Toybox.Lang.Boolean { return 2 + 3 * 4 >= 14; }").unwrap();
    assert!(matches!(
        function
            .operations()
            .iter()
            .find(|op| matches!(op.operation, Operation::Compare { .. }))
            .unwrap()
            .operation,
        Operation::Compare {
            kind: ComparisonOp::GreaterEqual,
            ..
        }
    ));
}
