use ciq_compiler::{
    ReturnType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources_with_api, compile_with_api, parse_ir, parse_ir_with_api,
    parse_unit_ir_with_api,
};

#[test]
fn wide_string_and_char_operators_preserve_complete_pipelines() {
    let context = ApiContext::new(ApiLevel(3, 1, 0), ApiLevel(3, 1, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[include_str!(
            "../../../examples/compiler-wide-operators/app.mc"
        )],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [92; 16],
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
    for (expression, ty, instruction) in [
        ("\"n=\" + 4294967297l", "String", "addv"),
        ("1.25 + \"f\"", "String", "addv"),
        ("\"d=\" + 1.25d", "String", "addv"),
        ("'A' + 1l", "Char", "addv"),
        ("1l + 'A'", "Char", "addv"),
        ("'A' == 65l", "Boolean", "eq"),
        ("65.0 == 'A'", "Boolean", "eq"),
        ("'A' != 66.0d", "Boolean", "ne"),
    ] {
        let f = compile(&format!(
            "function f() as Toybox.Lang.{ty} {{ return {expression}; }}"
        ))
        .unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        let text = f.lower().to_text();
        assert!(text.lines().any(|line| line == instruction));
        assert!(
            !text.contains("__ciq_native_"),
            "operator emitted a conversion call: {text}"
        );
    }
}

#[test]
fn exact_string_accepts_all_numeric_widths_without_proving_non_null() {
    for ty in ["Long", "Float", "Double"] {
        for expression in ["\"v=\" + n", "n + \"v\""] {
            let source = format!(
                "function f(n as Toybox.Lang.{ty} or Null) as Toybox.Lang.String {{ return {expression}; }}"
            );
            let f = compile(&source).unwrap();
            assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        }
        for expression in ["\"v\" == n", "n != \"v\""] {
            let f = compile(&format!("function f(n as Toybox.Lang.{ty} or Null) as Toybox.Lang.Boolean {{ return {expression}; }}")).unwrap();
            assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        }
        for body in [
            "System.println(\"v=\" + n); return n.toString();",
            "if (\"v\" == n) { return \"equal\"; } return n.toString();",
            "if (n != \"v\") { return n.toString(); } return \"equal\";",
        ] {
            assert!(
                compile(&format!(
                    "function f(n as Toybox.Lang.{ty} or Null) as Toybox.Lang.String {{ {body} }}"
                ))
                .is_err(),
                "{ty}: {body}"
            );
        }
        assert!(compile(&format!("function f(s as Toybox.Lang.String or Null, n as Toybox.Lang.{ty} or Null) as Toybox.Lang.String {{ return s + n; }}")).is_err());
    }
}

#[test]
fn char_addition_equality_and_ordering_have_distinct_profiles() {
    for literal in ["65l", "65.0", "65.0d"] {
        for expression in [format!("'A' == {literal}"), format!("{literal} != 'A'")] {
            let f = compile(&format!(
                "function f() as Toybox.Lang.Boolean {{ return {expression}; }}"
            ))
            .unwrap();
            assert_eq!(f.return_type(), ReturnType::Boolean);
        }
        for operator in ["<", "<=", ">", ">="] {
            for expression in [
                format!("'A' {operator} {literal}"),
                format!("{literal} {operator} 'A'"),
            ] {
                assert!(
                    compile(&format!(
                        "function f() as Toybox.Lang.Boolean {{ return {expression}; }}"
                    ))
                    .is_err()
                );
            }
        }
    }
    for expression in [
        "'A' + 1.0",
        "1.0 + 'A'",
        "'A' + 1.0d",
        "1.0d + 'A'",
        "'A' - 1l",
        "1l - 'A'",
        "'A' * 1l",
        "'A' / 1l",
        "'A' % 1l",
        "'A' == true",
    ] {
        assert!(
            compile(&format!(
                "function f() as Void {{ System.println({expression}); }}"
            ))
            .is_err(),
            "{expression}"
        );
    }
    for expression in ["'A' + n", "n < 'A'"] {
        assert!(
            compile(&format!(
                "function f(n as Toybox.Lang.Long or Null) as Void {{ System.println({expression}); }}"
            ))
            .is_err()
        );
    }
    for (source, replacement) in [
        (
            "function f() as Toybox.Lang.Char { return 'A' + 1l; }",
            (".long %1 1", ".float %1 0x3f800000"),
        ),
        (
            "function f() as Toybox.Lang.Boolean { return 'A' == 65.0; }",
            (".equal", ".less"),
        ),
        (
            "function f() as Toybox.Lang.String { return \"v\" + 1.0d; }",
            (".add", ".subtract"),
        ),
    ] {
        let text = compile(source).unwrap().to_ir_text();
        let edited = text.replace(replacement.0, replacement.1);
        assert_ne!(text, edited);
        assert!(parse_ir(&edited).is_err());
    }
}

#[test]
fn char_weak_retains_char_and_checks_receiver_availability() {
    let source = "function f(x as Toybox.Lang.Char or Null) as Toybox.Lang.Char { if (x == null) { return 'A'; } return x.weak(); }";
    let context = ApiContext::new(ApiLevel(1, 3, 0), ApiLevel(1, 3, 0)).unwrap();
    let f = compile_with_api(source, context).unwrap();
    assert_eq!(f, parse_ir_with_api(&f.to_ir_text(), context).unwrap());
    assert!(
        f.lower()
            .to_text()
            .contains("getsv @__ciq_native_weak\ninvokemz")
    );
    let api = ciq_compiler::api::core();
    let weak = api
        .method_for_type(&ciq_compiler::api::Type::named("Toybox.Lang.Char"), "weak")
        .unwrap();
    assert_eq!(
        weak.result,
        ciq_compiler::api::Type::named("Toybox.Lang.Char")
    );
    for context in [
        ApiContext::new(ApiLevel(1, 2, 0), ApiLevel(1, 2, 0)).unwrap(),
        ApiContext::new(ApiLevel(1, 3, 0), ApiLevel(1, 2, 0)).unwrap(),
    ] {
        assert!(compile_with_api(source, context).is_err());
        assert!(parse_ir_with_api(&f.to_ir_text(), context).is_err());
    }
    for source in [
        "function f(x as Toybox.Lang.Char or Null) as Toybox.Lang.Char { return x.weak(); }",
        "function f() as Toybox.Lang.Char { return 'A'.weak(1); }",
        "function f() as Toybox.Lang.Number { return 'A'.weak(); }",
    ] {
        assert!(compile(source).is_err());
    }
    for expression in ["'A'.weak().toNumber()", "('A'.weak() + 1l).toNumber()"] {
        let f = compile(&format!(
            "function f() as Toybox.Lang.Number {{ return {expression}; }}"
        ))
        .unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
}
