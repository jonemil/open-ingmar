use ciq_compiler::{ReturnType, compile, parse_ir};

#[test]
fn logical_keywords_preserve_symbolic_semantics_spans_and_nullable_type_syntax() {
    for (keywords, symbolic) in [
        ("a and b", "a && b"),
        ("a or b and a", "a || b && a"),
        ("a && b or a and b", "a && b || a && b"),
    ] {
        let source = format!(
            "function f(a as Toybox.Lang.Boolean, b as Toybox.Lang.Boolean) as Toybox.Lang.Boolean {{ return {keywords}; }}"
        );
        let f = compile(&source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        assert_eq!(
            f.lower(),
            compile(&source.replace(keywords, symbolic))
                .unwrap()
                .lower()
        );
        for op in f.operations() {
            if let Some(span) = op.span {
                assert!(source.get(span.start..span.end).is_some());
            }
        }
    }
    for source in [
        "function f(n as Toybox.Lang.Number or Null, b as Toybox.Lang.Boolean) { return n != null and b ? n : 0; }",
        "function f(s as Toybox.Lang.String or Null) { if (s == null or s.length() > 0) { return 1; } return 0; }",
        "function f(order, candy) { return order + candy; }",
    ] {
        let f = compile(source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    for source in [
        "function f() { var and = 1; return and; }",
        "function f(or) { return or; }",
        "function and() { return 1; }",
        "function f() { return 1 and true; }",
        "function f() { return true or 1; }",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
}

#[test]
fn number_bang_preserves_number_contract_and_uses_the_existing_inversion_opcode() {
    for operand in ["0", "1", "(-1)", "2147483647", "(-2147483648)", "n", "(!n)"] {
        let f = compile(&format!("function f(n) {{ return !{operand}; }}")).unwrap();
        assert_eq!(f.return_type(), ReturnType::Integer);
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        let complement = compile(&format!("function f(n) {{ return ~{operand}; }}")).unwrap();
        assert_eq!(f.lower(), complement.lower());
    }
    let f = compile("function f(n as Toybox.Lang.Number or Null) { return n != null ? !n : 0; }")
        .unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    for source in [
        "function f(n as Toybox.Lang.Number or Null) { return !n; }",
        "function f() { if (!1) { return 1; } return 0; }",
        "function f() { return !1 ? 1 : 2; }",
        "function f() { return !'a'; }",
        "function f() { return !null; }",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    let boolean = compile("function f() as Toybox.Lang.Boolean { return !true; }").unwrap();
    assert_eq!(boolean.return_type(), ReturnType::Boolean);
}
