use ciq_compiler::{
    ReturnType,
    api::{ApiContext, ApiLevel},
    compile, compile_with_api, parse_ir, parse_ir_with_api,
};

#[test]
fn integer_bitwise_pairs_promote_through_native_and_saved_ir() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    for (left, right) in [("Long", "Long"), ("Long", "Number"), ("Number", "Long")] {
        for (operator, opcode) in [
            ("&", "andv"),
            ("|", "orv"),
            ("^", "xorv"),
            ("<<", "shlv"),
            (">>", "shrv"),
        ] {
            let source = format!(
                "function f(left as Toybox.Lang.{left}, right as Toybox.Lang.{right}) as Toybox.Lang.Long {{ return left {operator} right; }}"
            );
            let f = compile_with_api(&source, initial).unwrap();
            assert_eq!(f.return_type(), ReturnType::Long);
            assert_eq!(f, parse_ir_with_api(&f.to_ir_text(), initial).unwrap());
            let assembly = f.lower();
            assert!(assembly.to_text().contains(opcode));
            assert_eq!(
                assembly,
                ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
            );
            assert!(
                compile(&source.replace(
                    "as Toybox.Lang.Long { return",
                    "as Toybox.Lang.Number { return"
                ))
                .is_err()
            );
        }
    }
}

#[test]
fn long_inversion_and_statement_updates_keep_their_type() {
    for operator in ["!", "~"] {
        let source = format!(
            "function f(value as Toybox.Lang.Long) as Toybox.Lang.Long {{ return {operator}value; }}"
        );
        let f = compile(&source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        assert!(f.lower().to_text().contains("invv"));
        assert!(
            compile(&source.replace(
                "as Toybox.Lang.Long { return",
                "as Toybox.Lang.Boolean { return"
            ))
            .is_err()
        );
    }
    for update in [
        "value++",
        "++value",
        "value--",
        "--value",
        "value &= 3",
        "value |= 1l",
        "value ^= 2",
        "value <<= 32",
        "value >>= 1l",
    ] {
        let source = format!(
            "function f() as Toybox.Lang.Long {{ var value = 4294967297l; {update}; return value; }}"
        );
        let f = compile(&source).unwrap();
        let text = f.to_ir_text();
        assert_eq!(f, parse_ir(&text).unwrap());
        // A changed local initializer must not disguise a width-changing write.
        if update.contains("1l") {
            let edited = text.replace(".long %0 4294967297", ".integer %0 1");
            assert_ne!(text, edited);
            assert!(parse_ir(&edited).is_err());
        }
    }
}

#[test]
fn bitwise_profiles_reject_invalid_counts_truthiness_and_update_expressions() {
    for source in [
        "function f() as Toybox.Lang.Long { return 1l << true; }",
        "function f() as Toybox.Lang.Long { return 1l & 'A'; }",
        "function f() as Toybox.Lang.Long { return 1l | null; }",
        "function f() as Toybox.Lang.Long { return 1l ^ \"one\"; }",
        "function f() as Toybox.Lang.Number { if (!1l) { return 1; } return 0; }",
        "function f() as Toybox.Lang.Number { var value = 1; value <<= 1l; return value; }",
        "function f() as Toybox.Lang.Long { var value = 1l; return value++; }",
        "function f() as Toybox.Lang.Long { var value = 1l; return ++value; }",
        "function f() as Toybox.Lang.Long { var value = 1l; var copy = value--; return copy; }",
        "function f() as Toybox.Lang.Long { return ~null; }",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    let source = "function f(value as Toybox.Lang.Number or Null) as Toybox.Lang.Long { if (value == null) { return 0l; } return 1l << value; }";
    let f = compile(source).unwrap();
    let text = f.to_ir_text();
    assert_eq!(f, parse_ir(&text).unwrap());
    assert!(parse_ir(&text.replace(".equal", ".not-equal")).is_err());
    assert!(compile(&source.replace("if (value == null) { return 0l; }", "")).is_err());
}
