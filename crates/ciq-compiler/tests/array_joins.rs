#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{
    Operation, ReturnType,
    api::{ApiContext, ApiLevel},
    compile_function, compile_function_with_api, parse_function_ir, parse_function_ir_with_api,
};

fn source(element: &str) -> String {
    format!(
        "function choose(flag as Toybox.Lang.Boolean,
            a as Toybox.Lang.Array<{element}>, b as Toybox.Lang.Array<{element}>)
            as Toybox.Lang.Array<{element}> {{ return flag ? a : b; }}"
    )
}

#[test]
fn every_supported_array_contract_roundtrips_through_a_stack_join() {
    for (class, tag) in [
        ("Number", "number"),
        ("Long", "long"),
        ("Float", "float"),
        ("Double", "double"),
        ("Char", "char"),
        ("Boolean", "boolean"),
        ("String", "string"),
        ("Symbol", "symbol"),
        ("Method", "method"),
        ("WeakReference", "weak-reference"),
        ("Comparable", "comparable"),
        ("Comparator", "comparator"),
    ] {
        for nullable in [false, true] {
            let element = format!(
                "Toybox.Lang.{class}{}",
                if nullable { " or Null" } else { "" }
            );
            let tag = format!("array-{}{tag}", if nullable { "nullable-" } else { "" });
            check_join(&source(&element), &tag);
        }
    }
    check_join(&source("Toybox.WatchUi.View"), "array-view");
}

fn check_join(source: &str, tag: &str) {
    let f = compile_function(source).unwrap_or_else(|e| panic!("{source}: {e:?}"));
    assert!(matches!(f.return_type(), ReturnType::Array(_)));
    assert!(f.to_ir_text().lines().any(
        |line| line.starts_with(".block-value ") && line.split_whitespace().nth(2) == Some(tag)
    ));
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    assert!(!f.operations().iter().any(|op| matches!(
        op.operation,
        Operation::ArrayNew { .. }
            | Operation::ArrayAllocate { .. }
            | Operation::LocalInitialize { .. }
            | Operation::LocalAssign { .. }
    )));
    let assembly = f.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    assembly.encode().unwrap();
}

#[test]
fn joins_preserve_invariance_without_implicit_container_conversion() {
    for source in [
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number> { return true ? [1] : [1l]; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number> { return true ? [true] : [1]; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>, b as Toybox.Lang.Array<Toybox.Lang.Number or Null>) as Toybox.Lang.Array<Toybox.Lang.Number> { return true ? a : b; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number or Null>) as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return true ? a : null; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Comparable>, b as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Array<Toybox.Lang.Number> { return true ? a : b; }",
    ] {
        assert!(checking::compile_function(source).is_err(), "{source}");
    }
    let text = compile_function(&source("Toybox.Lang.Number or Null"))
        .unwrap()
        .to_ir_text();
    for tag in [
        "array-char",
        "array-boolean",
        "nullable-array-number",
        "array-array-number",
        "array-dc",
    ] {
        let changed = text.replace("array-nullable-number", tag);
        assert_ne!(changed, text);
        assert!(parse_function_ir(&changed).is_err(), "{tag}");
    }
    let source = "function f() as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return true ? new[2] : []; }";
    let f = compile_function(source).unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
}

#[test]
fn new_join_tags_recheck_api_levels_in_source_and_saved_ir() {
    for (element, required) in [
        ("Toybox.Lang.Number or Null", ApiLevel(1, 0, 0)),
        ("Toybox.Lang.Char or Null", ApiLevel(1, 3, 0)),
        ("Toybox.Lang.WeakReference or Null", ApiLevel(1, 2, 0)),
        ("Toybox.Lang.Comparable or Null", ApiLevel(5, 0, 0)),
        ("Toybox.WatchUi.View", ApiLevel(1, 0, 0)),
    ] {
        let source = source(element);
        let context = ApiContext::new(required, required).unwrap();
        let f = compile_function_with_api(&source, context).unwrap();
        assert_eq!(
            f,
            parse_function_ir_with_api(&f.to_ir_text(), context).unwrap()
        );
        if required > ApiLevel(1, 0, 0) {
            let old = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
            assert!(compile_function_with_api(&source, old).is_err());
            assert!(parse_function_ir_with_api(&f.to_ir_text(), old).is_err());
        }
    }
}
