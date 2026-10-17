use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    compile, compile_sources, compile_with_api, parse_ir, parse_ir_with_api, parse_unit_ir,
};

#[test]
fn long_native_symbols_and_saved_ir_use_shared_dispatch() {
    for (expression, name, id) in [
        ("1l.abs()", "abs", 0x008000a6),
        ("1l.format(\"%d\")", "format", 0x0080009c),
        ("1l.toLong()", "toLong", 0x008000a9),
        ("1.toLong()", "toLong", 0x008000a9),
        ("1l.toNumber()", "toNumber", 0x008000a7),
        ("1l.toString()", "toString", 0x008000af),
        ("1l.equals(1)", "equals", 0x008000b3),
        ("1l.compareTo(true)", "compareTo", 0x00800e50),
        ("1l.hashCode()", "hashCode", 0x008000ad),
        ("1l.weak()", "weak", 0x00800418),
    ] {
        let f = compile(&format!("function f() {{ System.println({expression}); }}")).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        let symbols = ciq_assembly::Program::parse(&format!(
            ".symbol method \"{name}\" id=0x{id:08x}\n.symbol system \"Toybox_System\" id=0x00800005\n.symbol print \"println\" id=0x00800067"
        )).unwrap();
        let bindings = ciq_compiler::symbol_bindings(&f.lower(), &symbols).unwrap();
        assert_eq!(bindings[&format!("__ciq_native_{name}")], id);
    }
    for (method, id) in [("toLong()", 0x008000a9), ("toLongWithBase(16)", 0x00800b1c)] {
        let f = compile(&format!(
            "function f() as Toybox.Lang.Long or Null {{ return \"ff\".{method}; }}"
        ))
        .unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        let name = method.split('(').next().unwrap();
        let symbols =
            ciq_assembly::Program::parse(&format!(".symbol method \"{name}\" id=0x{id:08x}"))
                .unwrap();
        assert_eq!(
            ciq_compiler::symbol_bindings(&f.lower(), &symbols).unwrap()
                [&format!("__ciq_native_{name}")],
            id
        );
    }
}

#[test]
fn native_introductions_apply_to_source_and_saved_ir() {
    for (expression, introduced, before) in [
        ("1l.abs()", ApiLevel(1, 0, 0), None),
        ("1l.format(\"%d\")", ApiLevel(1, 0, 0), None),
        ("1l.toLong()", ApiLevel(1, 0, 0), None),
        ("1l.toNumber()", ApiLevel(1, 0, 0), None),
        ("1l.toString()", ApiLevel(1, 0, 0), None),
        ("1.toLong()", ApiLevel(1, 0, 0), None),
        ("1l.hashCode()", ApiLevel(1, 0, 0), None),
        ("1l.weak()", ApiLevel(1, 2, 0), Some(ApiLevel(1, 1, 9))),
        (
            "1l.equals(null)",
            ApiLevel(1, 3, 0),
            Some(ApiLevel(1, 2, 9)),
        ),
        (
            "1l.compareTo(1)",
            ApiLevel(5, 0, 0),
            Some(ApiLevel(4, 9, 9)),
        ),
    ] {
        let source = format!("function f() {{ System.println({expression}); }}");
        let context = ApiContext::new(introduced, introduced).unwrap();
        let f = compile_with_api(&source, context).unwrap();
        assert_eq!(f, parse_ir_with_api(&f.to_ir_text(), context).unwrap());
        if let Some(before) = before {
            for context in [
                ApiContext::new(before, before).unwrap(),
                ApiContext::new(introduced, before).unwrap(),
            ] {
                assert!(compile_with_api(&source, context).is_err(), "{expression}");
                assert!(parse_ir_with_api(&f.to_ir_text(), context).is_err());
            }
        }
    }
    for method in ["toLong()", "toLongWithBase(16)"] {
        let source =
            format!("function f() as Toybox.Lang.Long or Null {{ return \"42\".{method}; }}");
        let level = ApiLevel(3, 1, 0);
        let f = compile_with_api(&source, ApiContext::new(level, level).unwrap()).unwrap();
        let before = ApiContext::new(ApiLevel(3, 0, 9), ApiLevel(3, 0, 9)).unwrap();
        assert!(compile_with_api(&source, before).is_err());
        assert!(parse_ir_with_api(&f.to_ir_text(), before).is_err());
    }
}

#[test]
fn nullable_long_proofs_and_joins_survive_saved_ir() {
    for source in [
        "function f(x as Toybox.Lang.Long or Null) as Toybox.Lang.Long or Null { return x; }",
        "function f() as Toybox.Lang.Long or Null { return null; }",
        "function f(p as Toybox.Lang.Boolean) as Toybox.Lang.Long or Null { return p ? 1l : null; }",
        "function f(p as Toybox.Lang.Boolean) as Toybox.Lang.Long { return p ? 1l : 2l; }",
        "function f() as Toybox.Lang.Long { var n = \"42\".toLong(); if (n == null) { return 0l; } return n.abs(); }",
        "function f() as Toybox.Lang.Long { var n = \"42\".toLong(); while (n == null) { n = \"7\".toLong(); } return n.weak(); }",
        "function f(s as Toybox.Lang.String or Null) as Toybox.Lang.Long or Null { if (s == null) { return null; } return s.toLongWithBase(16); }",
        "function f(x as Toybox.Lang.Long or Null) as Toybox.Lang.Boolean { return 1.equals(x); }",
        "function f() as Toybox.Lang.Boolean { return true.compareTo(1l) < 1; }",
        "function f() as Toybox.Lang.Boolean { return 'A'.equals(65l); }",
    ] {
        let f = compile(source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap(), "{source}");
    }
    let unit = compile_sources(&["module M { function parse() as Toybox.Lang.Long or Null { return \"42\".toLong(); } function f() as Toybox.Lang.Long or Null { return M.parse(); } }"]).unwrap();
    let text = unit.to_ir_text();
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    let edited = text.replace("parse result=nullable-long", "parse result=long");
    assert_ne!(text, edited);
    assert!(parse_unit_ir(&edited).is_err());
}

#[test]
fn nullable_long_and_native_calls_cannot_bypass_contracts() {
    for source in [
        "function f() as Toybox.Lang.Long { return \"42\".toLong(); }",
        "function f() { System.println(\"42\".toLong()); }",
        "function f() as Toybox.Lang.Long { var n = \"42\".toLong(); return n + 1l; }",
        "function f() as Toybox.Lang.Long { var n = \"42\".toLong(); if (n == null) { return 0l; } n = null; return n.abs(); }",
        "function f() as Toybox.Lang.Long { var n = \"42\".toLong(); var copy = n; if (n == null) { return 0l; } return copy; }",
        "function f() as Toybox.Lang.Long { var n = \"42\".toLong(); while (n == null) { break; } return n; }",
        "function f() as Toybox.Lang.Long { var n = \"42\".toLong(); if (n == null) { return 0l; } n = 1; return n; }",
        "function f() as Toybox.Lang.Long { return 1l.abs(1); }",
        "function f() { return 1l.format(1); }",
        "function f() { return 1l.toNumber(1); }",
        "function f() { return 1l.compareTo(null); }",
        "function f() { return 1l.compareTo(\"1\"); }",
        "function f() { return 1l.toFloat(1); }",
        "function f() { return 1l.toDouble(1); }",
        "function f() { var n = \"42\".toLongWithBase(16l); }",
        "function f() { var n = \"42\".toLongWithBase(null); }",
        "function f() { var n = \"42\".toLong(16); }",
        "function f() as Toybox.Lang.Long { return true ? 1l : 1; }",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
}
