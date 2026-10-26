#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    compile_function, compile_function_with_api, parse_function_ir, parse_function_ir_with_api,
};

#[test]
fn enumeration_preserves_generic_elements_through_saved_stages() {
    for key in ["Number", "String", "Boolean", "Char"] {
        for value in [
            "Number",
            "Number or Null",
            "String",
            "String or Null",
            "Boolean",
            "Boolean or Null",
            "Char",
            "Char or Null",
        ] {
            for (method, element) in [("keys", key), ("values", value)] {
                let source = format!(
                    "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.{key}, \
                     Toybox.Lang.{value}>) as Toybox.Lang.Array<Toybox.Lang.{element}> \
                     {{ return d.{method}(); }}"
                );
                let f = compile_function(&source).unwrap();
                let text = f.to_ir_text();
                assert_eq!(f, parse_function_ir(&text).unwrap());
                let program = f.lower();
                assert_eq!(
                    program,
                    ciq_assembly::Program::parse(&program.to_text()).unwrap()
                );
                assert!(
                    program
                        .to_text()
                        .contains(&format!("__ciq_native_{method}"))
                );
                assert!(
                    parse_function_ir(
                        &text.replace(&format!(" {method}"), &format!(" {method} %0"))
                    )
                    .is_err()
                );
            }
        }
    }
}

#[test]
fn enumeration_rejects_erasure_wrong_arity_and_incompatible_results() {
    for source in [
        "function f(d as Toybox.Lang.Dictionary) as Toybox.Lang.Array<Toybox.Lang.Number> { return d.keys(); }",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number>) { return d.values(1).size(); }",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number>) as Toybox.Lang.Array<Toybox.Lang.Number> { return d.keys(); }",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number or Null>) as Toybox.Lang.Array<Toybox.Lang.Number> { return d.values(); }",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.String or Null>) { return d.values()[0].length(); }",
    ] {
        assert!(
            checking::compile_function(source).is_err(),
            "accepted {source}"
        );
    }
    let f = compile_function("function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number>) { return d.keys()[0].length(); }").unwrap();
    assert!(
        parse_function_ir(&f.to_ir_text().replace(" keys", " values")).is_err(),
        "{}",
        f.to_ir_text()
    );
}

#[test]
fn enumeration_is_available_at_initial_api_but_array_consumers_keep_their_minimum() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    for method in ["keys", "values"] {
        let source = format!("function f() {{ return {{1 => 2}}.{method}().size(); }}");
        let f = compile_function_with_api(&source, initial).unwrap();
        assert_eq!(
            f,
            parse_function_ir_with_api(&f.to_ir_text(), initial).unwrap()
        );
        let source = format!("function f() {{ return {{1 => 2}}.{method}().indexOf(1); }}");
        let f = compile_function(&source).unwrap();
        assert!(compile_function_with_api(&source, initial).is_err());
        assert!(parse_function_ir_with_api(&f.to_ir_text(), initial).is_err());
    }
}

#[test]
fn enumeration_resolves_native_symbols_from_assembly_input() {
    let symbols = ciq_assembly::Program::parse(
        ".symbol keys \"keys\" id=0x008000b9\n.symbol values \"values\" id=0x008000ba",
    )
    .unwrap();
    for (method, id) in [("keys", 0x008000b9), ("values", 0x008000ba)] {
        let f = compile_function(&format!(
            "function f() as Toybox.Lang.Array<Toybox.Lang.Number> {{ return {{1 => 2}}.{method}(); }}"
        ))
        .unwrap();
        let bindings = ciq_compiler::symbol_bindings(&f.lower(), &symbols).unwrap();
        assert_eq!(bindings[&format!("__ciq_native_{method}")], id);
    }
}
