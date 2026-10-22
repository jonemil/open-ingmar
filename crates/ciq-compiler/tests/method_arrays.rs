use ciq_compiler::{
    Operation,
    api::{ApiContext, ApiLevel},
    compile_function, compile_function_with_api, compile_sources, parse_function_ir, parse_unit_ir,
};

#[test]
fn method_arrays_preserve_signatures_construction_and_saved_stages() {
    let initial = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap();
    for ty in ["Toybox.Lang.Method", "Toybox.Lang.Method or Null"] {
        for body in ["return a;", "return [];", "return flag ? a : a;"] {
            let source = format!(
                "function f(a as Toybox.Lang.Array<{ty}>,flag as Toybox.Lang.Boolean) as Toybox.Lang.Array<{ty}> {{{body}}}"
            );
            let f = compile_function_with_api(&source, initial).unwrap();
            assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        }
    }
    for source in [
        "function f(m as Toybox.Lang.Method) as Toybox.Lang.Array<Toybox.Lang.Method> {return [m,m];}",
        "function f(m as Toybox.Lang.Method) as Toybox.Lang.Array<Toybox.Lang.Method or Null> {return [m,null];}",
    ] {
        let f = compile_function(source).unwrap();
        assert!(
            f.operations()
                .iter()
                .any(|op| matches!(op.operation, Operation::ArrayBegin { .. }))
        );
        assert!(
            !f.operations()
                .iter()
                .any(|op| matches!(op.operation, Operation::ArrayConstant { .. }))
        );
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        let asm = f.lower();
        assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    }
    let source = "module M {function one(m as Toybox.Lang.Method) as Toybox.Lang.Array<Toybox.Lang.Method>{return [m];}function pass(m as Toybox.Lang.Method) as Toybox.Lang.Array<Toybox.Lang.Method>{return M.one(m);}}";
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains("result=method-array"));
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    assert!(
        parse_unit_ir(&text.replace("result=method-array", "result=nullable-method-array"))
            .is_err()
    );
}

#[test]
fn nullable_indexed_methods_require_local_snapshots() {
    let ty = "Toybox.Lang.Array<Toybox.Lang.Method or Null>";
    for body in [
        "var saved=a[0];a[0]=null;if(saved!=null){saved.invoke();}",
        "var saved=a[0];while(saved!=null){saved.invoke();saved=null;}",
        "a.add(null);a.add(m);a[0]=m;a.remove(null);a.removeAll(m);",
        "var b=a.reverse();var c=b.slice(0,b.size());var saved=c[0];if(null!=saved){saved.invoke();}",
    ] {
        let source = format!("function f(a as {ty},m as Toybox.Lang.Method) as Void {{{body}}}");
        let f = compile_function(&source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    for body in [
        "a[0].invoke();",
        "if(a[0]!=null){a[0].invoke();}",
        "var saved=a[0];if(saved!=null){saved=null;saved.invoke();}",
    ] {
        let source = format!("function f(a as {ty}) as Void {{{body}}}");
        assert!(compile_function(&source).is_err(), "accepted {source}");
    }
    let source = format!(
        "function f(a as {ty}) as Void {{var saved=a[0];if(saved!=null){{saved.invoke();}}}}"
    );
    let text = compile_function(&source).unwrap().to_ir_text();
    assert!(parse_function_ir(&text.replace(".not-equal ", ".equal ")).is_err());
}

#[test]
fn method_array_invariance_and_payload_validation_survive_ir_reload() {
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Method>) as Void {a.add(null);}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Method>) as Void {a[0]=1;}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Method>) as Toybox.Lang.Array<Toybox.Lang.Method or Null> {return a;}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Method or Null>) as Toybox.Lang.Array<Toybox.Lang.Method> {return a;}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Method>,b as Toybox.Lang.Array<Toybox.Lang.Method or Null>) as Void {a.addAll(b);}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Method>) as Void {a.sort(null);}",
    ] {
        assert!(compile_function(source).is_err(), "accepted {source}");
    }
    let source = "function f() as Toybox.Lang.Array<Toybox.Lang.Method or Null> {return [null];}";
    let f = compile_function(source).unwrap();
    assert!(
        f.operations()
            .iter()
            .any(|op| matches!(op.operation, Operation::ArrayConstant { .. }))
    );
    let text = f.to_ir_text();
    assert_eq!(f, parse_function_ir(&text).unwrap());
    for edited in [
        text.replace("nullable-method null", "method null"),
        text.replace("nullable-method null", "nullable-method 0x00000001"),
    ] {
        assert_ne!(text, edited);
        assert!(parse_function_ir(&edited).is_err());
    }
}
