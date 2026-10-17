use ciq_compiler::{
    ReturnType,
    api::{ApiContext, ApiLevel},
    compile, compile_sources, compile_with_api, parse_ir, parse_unit_ir,
};

#[test]
fn integer_arithmetic_promotes_only_at_the_operator_and_preserves_saved_stages() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    for (left, right) in [("Long", "Long"), ("Long", "Number"), ("Number", "Long")] {
        for (operator, mnemonic) in [
            ("+", "addv"),
            ("-", "subv"),
            ("*", "mulv"),
            ("/", "divv"),
            ("%", "modv"),
        ] {
            let source = format!(
                "function f(left as Toybox.Lang.{left}, right as Toybox.Lang.{right}) as Toybox.Lang.Long {{ return left {operator} right; }}"
            );
            let f = compile_with_api(&source, initial).unwrap();
            assert_eq!(f.return_type(), ReturnType::Long);
            assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
            let assembly = f.lower();
            assert_eq!(
                assembly,
                ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
            );
            assert!(assembly.to_text().contains(mnemonic));
            let source = source.replace(
                "as Toybox.Lang.Long { return",
                "as Toybox.Lang.Number { return",
            );
            assert!(compile(&source).is_err());
        }
        for operator in ["==", "!=", "<", ">", "<=", ">="] {
            let source = format!(
                "function f(left as Toybox.Lang.{left}, right as Toybox.Lang.{right}) as Toybox.Lang.Boolean {{ return left {operator} right; }}"
            );
            let f = compile_with_api(&source, initial).unwrap();
            assert_eq!(f.return_type(), ReturnType::Boolean);
            assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        }
    }
    let source = "module A { function take(value as Toybox.Lang.Long) as Toybox.Lang.Long { return value; } function f() as Toybox.Lang.Long { return A.take(1l + 2); } }";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert!(compile_sources(&[&source.replace("1l + 2", "1 + 2")]).is_err());
}

#[test]
fn long_signs_select_long_zero_and_preserve_operand_effects() {
    for operator in ["+", "-"] {
        let f = compile(&format!("function f(value as Toybox.Lang.Long) as Toybox.Lang.Long {{ return {operator}value; }}")).unwrap();
        let text = f.to_ir_text();
        assert_eq!(f, parse_ir(&text).unwrap());
        let assembly = f.lower().to_text();
        if operator == "-" {
            assert!(assembly.contains("lpushz"));
            assert!(assembly.contains("subv"));
        } else {
            assert!(!assembly.contains("addv"));
            assert!(!assembly.contains("lpushz"));
        }
        let edited = text.replace("parameter-types=long", "parameter-types=string");
        assert_ne!(text, edited);
        assert!(parse_ir(&edited).is_err());
    }
    let f = compile("function f() as Toybox.Lang.Long { var value = 1l; value += 2; value *= 3l; return value; }").unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
}

#[test]
fn promotion_does_not_widen_storage_signatures_or_other_operator_profiles() {
    for source in [
        "function f() { var value = 1; value += 2l; return value; }",
        "function f() as Toybox.Lang.Long { var value = 1l; value = 1; return value; }",
        "function f() as Toybox.Lang.Long { return 1l + 'A'; }",
        "function f() as Toybox.Lang.Boolean { return 1l < 'A'; }",
        "function f() as Toybox.Lang.Long { return 1l + true; }",
        "function f() as Toybox.Lang.Long { return 1l + null; }",
        "function f() as Toybox.Lang.String { return \"value\" - 1l; }",
        "function f() as Toybox.Lang.Long { return 1l & false; }",
        "function f() as Toybox.Lang.Long { return 1l << false; }",
        "function f() as Toybox.Lang.Long { var value = 1l; return value++; }",
        "function f() { if (1l) { return 1; } return 0; }",
        "function f() as Toybox.Lang.Long { return true ? 1l : 2; }",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    let source = "function f(value as Toybox.Lang.Number or Null) as Toybox.Lang.Long { if (value == null) { return 0l; } return value + 1l; }";
    let f = compile(source).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    assert!(compile(&source.replace("if (value == null) { return 0l; }", "")).is_err());
}
