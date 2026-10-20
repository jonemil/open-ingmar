use ciq_compiler::{compile_sources, parse_ir, parse_unit_ir};

const CLASSES: &str = "class Token { function initialize() {} function value() { return 41; } } class Other { function initialize() {} function value() { return 9; } }";

fn check(source: &str) -> ciq_compiler::CompilationUnit {
    let unit = compile_sources(&[source, CLASSES]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert_eq!(
        unit.lower(),
        ciq_assembly::Program::parse(&unit.lower().to_text()).unwrap()
    );
    unit
}

#[test]
fn nullable_source_signatures_copies_calls_and_null_proofs_round_trip() {
    let unit = check(
        r#"
        module Tokens {
            function pass(token as Token or Null) as Token or Null { return token; }
            function exact(token as Token) as Token { return token; }
            function read(token as Token or Null) {
                var copy = Tokens.pass(token);
                if (copy != null) { return Tokens.exact(copy).value(); }
                return 0;
            }
            function empty() as Token or Null { var n = null; return n; }
            function run() {
                var n = null;
                Tokens.pass(n);
                Tokens.pass(true ? null : null);
                var token = Tokens.pass(new Token());
                var copy = token;
                if (!(copy == null)) { var exact = copy; Tokens.exact(exact); }
                copy = null;
                token = new Token();
                while (token != null) { Tokens.exact(token); token = null; }
                if (token != null and token.value() == 41) { return 1; }
                if (token == null or token.value() == 41) { return 2; }
                return 0;
            }
        }
        class Holder {
            function initialize() {}
            function pass(token as Token or Null) as Token or Null { return token; }
            function call(token as Token or Null) as Token or Null { return self.pass(token); }
        }
        function run() {
            var holder = new Holder();
            var token = holder.call(null);
            if (token instanceof Token) { return token.value(); }
            return Tokens.read(holder.pass(new Token()));
        }
    "#,
    );
    let text = unit.to_ir_text();
    assert!(text.contains("parameter-types=nullable-source(Token@Lang.Object)"));
    assert!(text.contains("result=nullable-source(Token@Lang.Object)"));
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
}

#[test]
fn nullable_source_contracts_reject_erasure_unrelated_classes_and_missing_proofs() {
    let prelude = "module Tokens { function pass(token as Token or Null) as Token or Null { return token; } function exact(token as Token) { return token.value(); } }";
    for source in [
        "function f(token as Token or Null) { return token.value(); }",
        "function f(token as Token or Null) as Token { return token; }",
        "function f(token as Token or Null) { return Tokens.exact(token); }",
        "function f(token as Token or Null) { var copy = token; if (token != null) { return copy.value(); } return 0; }",
        "function f(token as Token or Null) { var copy = token; if (copy != null) { copy = null; return copy.value(); } return 0; }",
        "function f(token as Token or Null) { var copy = token; if (copy != null) { var exact = copy; exact = null; } return 0; }",
        "function f(token as Token or Null) { var copy = token; if (copy != null) { copy = new Other(); } return 0; }",
        "function f(token as Token or Null) { var copy = token; var guarded = copy != null; if (guarded) { return copy.value(); } return 0; }",
        "function f(token as Token or Null) { var copy = token; while (copy != null) { copy = null; } return copy.value(); }",
        "function f(token as Token or Null) { var copy = token; if (true) { if (copy != null) { return 0; } } return copy.value(); }",
        "function f(token as Other or Null) { Tokens.pass(token); }",
        "function f(token as Toybox.Lang.Object or Null) { Tokens.pass(token); }",
        "function f(token as Other) as Token or Null { return token; }",
        "function f() as Token or Null { return new Other(); }",
        "function f(token as Missing or Null) { return 0; }",
        "function f() { var token = new Token(); token = null; }",
    ] {
        assert!(
            compile_sources(&[source, prelude, CLASSES]).is_err(),
            "{source}"
        );
    }
}

#[test]
fn saved_ir_cannot_forge_nullable_class_or_non_null_proofs() {
    let unit = check(
        "module Tokens { function pass(token as Token or Null) as Token or Null { return token; } function read(token as Token or Null) { if (token != null) { return token.value(); } return 0; } } function f() { return Tokens.read(Tokens.pass(new Token())); }",
    );
    let text = unit.to_ir_text();
    for edited in [
        text.replace(
            "nullable-source(Token@Lang.Object)",
            "nullable-source(Missing@Lang.Object)",
        ),
        text.replace(
            "Tokens pass result=nullable-source(Token@Lang.Object)",
            "Tokens pass result=source(Token@Lang.Object)",
        ),
        text.replace(".not-equal", ".equal"),
        text.replace(
            "nullable-source(Token@Lang.Object)",
            "nullable-source(Token@WatchUi.View)",
        ),
    ] {
        assert_ne!(text, edited);
        assert!(parse_unit_ir(&edited).is_err(), "{edited}");
    }
    for tag in [
        "nullable-source()",
        "nullable-source(Token@Application.AppBase)",
        "nullable-source(Token@Unknown)",
    ] {
        assert!(
            parse_ir(&format!(
                "ciq-ir\n.function f parameters=1 parameter-types={tag}\n.return-void\n.end\n"
            ))
            .is_err()
        );
    }
}
