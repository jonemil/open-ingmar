#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{
    Operation,
    api::{ApiContext, ApiLevel},
    compile_function, compile_function_with_api, compile_sources, parse_function_ir,
    parse_function_ir_with_api, parse_unit_ir,
};

#[test]
fn weak_container_contracts_survive_saved_stages_and_api_checks() {
    let api = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 2, 0)).unwrap();
    let old = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 1, 0)).unwrap();
    for ty in [
        "Toybox.Lang.Array<Toybox.Lang.WeakReference>",
        "Toybox.Lang.Array<Toybox.Lang.WeakReference or Null>",
        "Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.WeakReference>",
        "Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.WeakReference or Null>",
    ] {
        let source = format!("function f(a as {ty}) as {ty} {{return a;}}");
        let f = compile_function_with_api(&source, api).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        assert!(compile_function_with_api(&source, old).is_err());
        assert!(parse_function_ir_with_api(&f.to_ir_text(), old).is_err());
    }
    for ty in [
        "Toybox.Lang.WeakReference",
        "Toybox.Lang.WeakReference or Null",
    ] {
        let source = format!(
            "function f(a as Toybox.Lang.Array<{ty}>,b as Toybox.Lang.Array<{ty}>,flag as Toybox.Lang.Boolean) as Toybox.Lang.Array<{ty}> {{return flag?a:b;}}"
        );
        let f = compile_function(&source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    for source in [
        "function f(w as Toybox.Lang.WeakReference) as Toybox.Lang.Array<Toybox.Lang.WeakReference> {return [w,w];}",
        "function f(w as Toybox.Lang.WeakReference) as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.WeakReference> {var d={\"live\"=>w};return d;}",
    ] {
        let f = compile_function(source).unwrap();
        assert!(f.operations().iter().any(|op| matches!(
            op.operation,
            Operation::ArrayBegin { .. } | Operation::DictionaryBegin { .. }
        )));
        assert!(!f.operations().iter().any(|op| matches!(
            op.operation,
            Operation::ArrayConstant { .. } | Operation::DictionaryConstant { .. }
        )));
        let asm = f.lower();
        assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    let unit=compile_sources(&["module M {function make(w as Toybox.Lang.WeakReference) as Toybox.Lang.Array<Toybox.Lang.WeakReference>{return [w];}function forward(w as Toybox.Lang.WeakReference) as Toybox.Lang.Array<Toybox.Lang.WeakReference>{return M.make(w);}}"] ).unwrap();
    let text = unit.to_ir_text();
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    let edited = text.replace("result=weak-reference-array", "result=string-array");
    assert_ne!(text, edited);
    assert!(parse_unit_ir(&edited).is_err());
}

#[test]
fn weak_container_lookups_require_fresh_nullable_snapshots() {
    for (ty, read, clear) in [
        (
            "Toybox.Lang.Array<Toybox.Lang.WeakReference or Null>",
            "a[0]",
            "a[0]=null;",
        ),
        (
            "Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.WeakReference>",
            "a[\"live\"]",
            "a.remove(\"live\");",
        ),
        (
            "Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.WeakReference or Null>",
            "a.get(\"live\")",
            "a.put(\"live\",null);",
        ),
    ] {
        let source = format!(
            "function f(a as {ty}) as Void {{var saved={read};{clear}if(saved!=null){{saved.stillAlive();var target=saved.get();if(target!=null){{target.toString();}}}}}}"
        );
        let f = compile_function(&source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        assert!(parse_function_ir(&f.to_ir_text().replace(".not-equal ", ".equal ")).is_err());
        let unsafe_read =
            format!("function f(a as {ty}) as Void {{if({read}!=null){{{read}.stillAlive();}}}}");
        assert!(checking::compile_function(&unsafe_read).is_err());
    }
    let source = "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.WeakReference or Null>) as Toybox.Lang.Array<Toybox.Lang.WeakReference or Null> {return d.values();}";
    let f = compile_function(source).unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
}

#[test]
fn weak_container_invariance_and_null_constants_are_rechecked() {
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.WeakReference>) as Void {a.add(null);}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.WeakReference>) as Toybox.Lang.Array<Toybox.Lang.WeakReference or Null> {return a;}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.WeakReference>) as Void {a.sort(null);}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.WeakReference>) as Void {d.put(\"a\",null);}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.WeakReference>) as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.WeakReference or Null> {return d;}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.WeakReference,Toybox.Lang.Number>) as Void {}",
    ] {
        assert!(
            checking::compile_function(source).is_err(),
            "accepted {source}"
        );
    }
    for source in [
        "function f() as Toybox.Lang.Array<Toybox.Lang.WeakReference or Null> {return [null];}",
        "function f() as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.WeakReference or Null> {return {\"empty\"=>null};}",
    ] {
        let f = compile_function(source).unwrap();
        assert!(f.operations().iter().any(|op| matches!(
            op.operation,
            Operation::ArrayConstant { .. } | Operation::DictionaryConstant { .. }
        )));
        let text = f.to_ir_text();
        assert_eq!(f, parse_function_ir(&text).unwrap());
        let replacement = if text.contains(".array-constant") {
            " 0x00000001 span="
        } else {
            " number 0x00000001 span="
        };
        let edited = text.replace(" null span=", replacement);
        assert_ne!(text, edited);
        assert!(parse_function_ir(&edited).is_err());
    }
}
