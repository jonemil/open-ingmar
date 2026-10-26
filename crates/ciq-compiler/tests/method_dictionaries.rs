#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{
    Operation,
    api::{ApiContext, ApiLevel},
    compile_function, compile_function_with_api, compile_sources, parse_function_ir, parse_unit_ir,
};

#[test]
fn method_dictionary_contracts_preserve_signatures_and_runtime_literals() {
    let initial = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap();
    for (key, value) in [
        ("Method", "Number"),
        ("Method", "Method"),
        ("Method", "Method or Null"),
        ("String", "Method or Null"),
    ] {
        let ty = format!("Toybox.Lang.Dictionary<Toybox.Lang.{key},Toybox.Lang.{value}>");
        let source = format!("function f(d as {ty}) as {ty} {{return d;}}");
        let f = compile_function_with_api(&source, initial).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    for source in [
        "function f(m as Toybox.Lang.Method) as Toybox.Lang.Dictionary<Toybox.Lang.Method,Toybox.Lang.Method> {return {m=>m};}",
        "function f(m as Toybox.Lang.Method) as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.Method or Null> {return {\"m\"=>m,\"empty\"=>null};}",
        "function f(m as Toybox.Lang.Method) as Toybox.Lang.Dictionary<Toybox.Lang.Method,Toybox.Lang.Method> {var d={m=>m};return d;}",
    ] {
        let f = compile_function(source).unwrap();
        assert!(
            f.operations()
                .iter()
                .any(|op| matches!(op.operation, Operation::DictionaryBegin { .. }))
        );
        assert!(
            !f.operations()
                .iter()
                .any(|op| matches!(op.operation, Operation::DictionaryConstant { .. }))
        );
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        let assembly = f.lower();
        assert_eq!(
            assembly,
            ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
        );
    }
    for (member, result) in [("keys", "Method"), ("values", "Method or Null")] {
        let source = format!(
            "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.Method,Toybox.Lang.Method or Null>) as Toybox.Lang.Array<Toybox.Lang.{result}> {{return d.{member}();}}"
        );
        let f = compile_function(&source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
}

#[test]
fn nullable_method_keys_and_lookup_values_require_fresh_proofs() {
    let ty = "Toybox.Lang.Dictionary<Toybox.Lang.Method,Toybox.Lang.Method>";
    for action in [
        "var v=d[k];",
        "d[k]=m;",
        "d.get(k);",
        "d.put(k,m);",
        "d.hasKey(k);",
        "d.remove(k);",
    ] {
        let source = format!(
            "function f(d as {ty},k as Toybox.Lang.Method or Null,m as Toybox.Lang.Method) as Void {{{action}}}"
        );
        assert!(
            checking::compile_function(&source).is_err(),
            "accepted {source}"
        );
        let guarded = format!(
            "function f(d as {ty},k as Toybox.Lang.Method or Null,m as Toybox.Lang.Method) as Void {{if(k!=null){{{action}}}}}"
        );
        let f = compile_function(&guarded).unwrap_or_else(|e| panic!("{guarded}: {e}"));
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    for value in ["Method", "Method or Null"] {
        let ty = format!("Toybox.Lang.Dictionary<Toybox.Lang.Method,Toybox.Lang.{value}>");
        let source = format!(
            "function f(d as {ty},k as Toybox.Lang.Method) as Void {{var saved=d[k];d.remove(k);if(saved!=null){{saved.invoke();}}}}"
        );
        let f = compile_function(&source).unwrap();
        let text = f.to_ir_text();
        assert_eq!(f, parse_function_ir(&text).unwrap());
        assert!(parse_function_ir(&text.replace(".not-equal ", ".equal ")).is_err());
        let repeated = format!(
            "function f(d as {ty},k as Toybox.Lang.Method) as Void {{if(d[k]!=null){{d[k].invoke();}}}}"
        );
        assert!(checking::compile_function(&repeated).is_err());
    }
}

#[test]
fn dictionary_method_invariance_and_constant_validation_survive_saved_ir() {
    for source in [
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.Method,Toybox.Lang.Method>,m as Toybox.Lang.Method) as Void {d[m]=null;}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.Method,Toybox.Lang.Method>,m as Toybox.Lang.Method) as Void {d.put(1,m);}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.Method,Toybox.Lang.Method>) as Toybox.Lang.Dictionary<Toybox.Lang.Method,Toybox.Lang.Method or Null> {return d;}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.Method or Null,Toybox.Lang.Number>) as Void {}",
    ] {
        assert!(compile_function(source).is_err(), "accepted {source}");
    }
    let source = "function f() as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.Method or Null> {return {\"empty\"=>null};}";
    let f = compile_function(source).unwrap();
    assert!(
        f.operations()
            .iter()
            .any(|op| matches!(op.operation, Operation::DictionaryConstant { .. }))
    );
    let text = f.to_ir_text();
    assert_eq!(f, parse_function_ir(&text).unwrap());
    for edited in [
        text.replace(
            "dictionary(string=>nullable-method)",
            "dictionary(string=>method)",
        ),
        text.replace(" null", " number 0x00000001"),
    ] {
        assert_ne!(text, edited);
        assert!(parse_function_ir(&edited).is_err());
    }
    let source = "module M {function put(d as Toybox.Lang.Dictionary<Toybox.Lang.Method,Toybox.Lang.Method>,m as Toybox.Lang.Method) as Void{d.put(m,m);} function run(m as Toybox.Lang.Method) as Void {var d={m=>m};M.put(d,m);}}";
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    let edited = text.replace(
        "parameter-types=dictionary(method=>method):method",
        "parameter-types=dictionary(method=>nullable-method):method",
    );
    assert_ne!(text, edited);
    assert!(parse_unit_ir(&edited).is_err());
}
