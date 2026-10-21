use ciq_compiler::{JoinType, Operation, compile_sources, parse_ir, parse_unit_ir};

const CLASSES: &str = "class Token { function initialize() {} function value() { return 41; } } class Other { function initialize() {} function value() { return 9; } }";

fn checked(source: &str) -> ciq_compiler::CompilationUnit {
    let unit = compile_sources(&[source, CLASSES]).unwrap_or_else(|e| panic!("{source}: {e}"));
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert_eq!(
        unit.lower(),
        ciq_assembly::Program::parse(&unit.lower().to_text()).unwrap()
    );
    unit
}

#[test]
fn conditional_values_preserve_exact_class_nullability_and_nested_edges() {
    for source in [
        "function f(flag as Toybox.Lang.Boolean) { var t=flag?new Token():new Token(); return t.value(); }",
        "function f(flag as Toybox.Lang.Boolean) as Token or Null { return flag?new Token():null; }",
        "function f(flag as Toybox.Lang.Boolean) as Token or Null { return flag?null:new Token(); }",
        "function f(flag as Toybox.Lang.Boolean,t as Token or Null) as Token or Null { return flag?t:null; }",
        "function f(flag as Toybox.Lang.Boolean,t as Token or Null) as Token or Null { return flag?t:new Token(); }",
        "function f(flag as Toybox.Lang.Boolean,t as Token or Null) as Token or Null { return flag?t:(flag?new Token():null); }",
        "function f(flag as Toybox.Lang.Boolean,o as Toybox.Lang.Object) as Token or Null { return o instanceof Token ? (flag?o:new Token()):null; }",
        "function f(flag as Toybox.Lang.Boolean,t as Token or Null) { if(t!=null){var copy=flag?t:new Token();return copy.value();}return 0; }",
        "class Holder { var t as Token or Null; function initialize() {} function set(flag as Toybox.Lang.Boolean) {self.t=flag?new Token():null;} function get() as Token or Null {return self.t;} }",
        "function f(flag as Toybox.Lang.Boolean) {var t=flag?new Token():null; var copy=t; t=null; if(copy!=null){return copy.value();}return 0;}",
    ] {
        checked(source);
    }
    let unit = checked(
        "function f(flag as Toybox.Lang.Boolean) as Token or Null {return flag?new Token():null;}",
    );
    assert!(
        unit.declarations()[0]
            .function
            .operations()
            .iter()
            .any(|op| matches!(
                op.operation,
                Operation::BlockParameter {
                    value_type: JoinType::Source { nullable: true, .. },
                    ..
                }
            ))
    );
    assert!(
        unit.to_ir_text()
            .contains("nullable-source(Token@Lang.Object)")
    );
}

#[test]
fn source_join_rejects_mismatched_classes_erasure_and_lost_null_proofs() {
    for source in [
        "function f(flag as Toybox.Lang.Boolean) as Token {return flag?new Token():new Other();}",
        "function f(flag as Toybox.Lang.Boolean,o as Toybox.Lang.Object) as Token {return flag?new Token():o;}",
        "function f(flag as Toybox.Lang.Boolean) {var t=flag?new Token():null;return t.value();}",
        "function f(flag as Toybox.Lang.Boolean) as Token {return flag?new Token():null;}",
        "function f(flag as Toybox.Lang.Boolean) {var t=flag?new Token():null;if(t!=null){t=null;return t.value();}return 0;}",
        "function f(flag as Toybox.Lang.Boolean,t as Token or Null) {if(t!=null){t=null;var joined=flag?t:new Token();return joined.value();}return 0;}",
    ] {
        assert!(compile_sources(&[source, CLASSES]).is_err(), "{source}");
    }
    checked(
        "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Object {return flag?(new Token() as Toybox.Lang.Object):(new Other() as Toybox.Lang.Object);}",
    );
}

#[test]
fn saved_join_edges_cannot_forge_class_or_non_null_contracts() {
    let unit = checked(
        "function f(flag as Toybox.Lang.Boolean) as Token or Null {return flag?new Token():null;}",
    );
    let text = unit.to_ir_text();
    for target in [
        "source(Token@Lang.Object)",
        "nullable-source(Other@Lang.Object)",
        "nullable-source(Token@WatchUi.View)",
        "nullable-source(Missing@Lang.Object)",
        "nullable-source()",
    ] {
        let changed = text
            .lines()
            .map(|line| {
                if line.starts_with(".block-value ") {
                    line.replace("nullable-source(Token@Lang.Object)", target)
                } else {
                    line.into()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_ne!(text, changed);
        assert!(parse_unit_ir(&changed).is_err(), "{changed}");
    }
    let unknown = "ciq-ir\n.function f result=nullable-object\n.block ^0\n.null %0\n.jump-value ^1 %0\n.block ^1\n.block-value %1 nullable-source(Missing@Lang.Object)\n.return %1\n.end\n";
    // A standalone function retains unit obligations; a Null edge is compatible
    // with a nullable declaration, but the enclosing unit must resolve its class.
    parse_ir(unknown).unwrap();
    assert!(parse_unit_ir(&format!("ciq-unit\n.owner global\n{unknown}")).is_err());
}
