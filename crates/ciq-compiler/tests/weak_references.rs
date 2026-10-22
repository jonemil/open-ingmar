use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_function_with_api, compile_sources, parse_function_ir,
    parse_function_ir_with_api, parse_unit_ir,
};

#[test]
fn weak_references_and_opaque_objects_keep_saved_contracts_and_api_checks() {
    let api = ApiContext::new(ApiLevel(1, 2, 0), ApiLevel(1, 2, 0)).unwrap();
    let old = ApiContext::new(ApiLevel(1, 2, 0), ApiLevel(1, 1, 0)).unwrap();
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.WeakReference { return a.weak(); }",
        "function f(d as Toybox.Lang.Dictionary) as Toybox.Lang.WeakReference { return d.weak(); }",
        "function f(w as Toybox.Lang.WeakReference) as Toybox.Lang.Object or Null { return w.get(); }",
        "function f(w as Toybox.Lang.WeakReference) as Toybox.Lang.Boolean { return w.stillAlive(); }",
        "function f(w as Toybox.Lang.WeakReference) as Toybox.Lang.WeakReference { return w.weak(); }",
        "function f(w as Toybox.Lang.WeakReference) as Toybox.Lang.Boolean { return w.equals(w); }",
        "function f(w as Toybox.Lang.WeakReference) { return w.hashCode(); }",
        "function f(w as Toybox.Lang.WeakReference) as Toybox.Lang.String { return w.toString(); }",
        "function f(w as Toybox.Lang.WeakReference) as Toybox.Lang.String { var v=w.get(); if(v==null) { return \"expired\"; } return v.toString(); }",
    ] {
        let f = compile_function_with_api(source, api).unwrap();
        let text = f.to_ir_text();
        assert_eq!(f, parse_function_ir_with_api(&text, api).unwrap());
        assert!(compile_function_with_api(source, old).is_err());
        assert!(parse_function_ir_with_api(&text, old).is_err());
        let asm = f.lower();
        assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    }
    for source in [
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Object { return o; }",
        "function f(o as Toybox.Lang.Object or Null) as Toybox.Lang.String { var v=o; while(v==null) { v=1; } return v.toString(); }",
        "function f(o as Toybox.Lang.Object or Null) as Toybox.Lang.Object or Null { var v=o; v=1; v=null; return v; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Object { return a; }",
        "function f(o as Toybox.Lang.Object, b as Toybox.Lang.Boolean) as Toybox.Lang.Object or Null { return b ? o : null; }",
        "function f(a as Toybox.Lang.WeakReference,b as Toybox.Lang.WeakReference,flag as Toybox.Lang.Boolean) as Toybox.Lang.WeakReference { return flag ? a : b; }",
    ] {
        let f = compile_function(source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
}

#[test]
fn weak_retrieval_requires_a_saved_non_null_proof_and_preserves_opacity() {
    for body in [
        "return w.get().toString();",
        "if(w.stillAlive()) { return w.get().toString(); } return \"x\";",
        "if(w.get()!=null) { return w.get().toString(); } return \"x\";",
        "var v=w.get(); if(v!=null) { v=null; return v.toString(); } return \"x\";",
        "var v=w.get(); if(v!=null) { return v.size(); } return \"x\";",
        "return w.get(1);",
        "return w.stillAlive(1);",
        "return w.clear();",
        "return w.get().weak();",
    ] {
        assert!(
            compile_function(&format!(
                "function f(w as Toybox.Lang.WeakReference) as Toybox.Lang.String {{ {body} }}"
            ))
            .is_err(),
            "accepted {body}"
        );
    }
    for source in [
        "function f(w as Toybox.Lang.WeakReference) as Toybox.Lang.Array<Toybox.Lang.Number> { var v=w.get(); if(v==null) { return []; } return v; }",
        "function f(w as Toybox.Lang.WeakReference) { w=null; return 0; }",
        "function f(o as Toybox.Lang.Object) { return o.weak(); }",
        "function f() as Toybox.Lang.Object { return null; }",
    ] {
        assert!(compile_function(source).is_err(), "accepted {source}");
    }
    let f = compile_function("function f(w as Toybox.Lang.WeakReference) as Toybox.Lang.Object or Null { return w.get(); }").unwrap();
    let text = f.to_ir_text();
    assert!(parse_function_ir(&text.replace("result=nullable-object", "result=object")).is_err());
    assert!(parse_function_ir(&text.replace(" get", " stillAlive")).is_ok()); // Boolean may erase to Object.
    assert!(parse_function_ir(&text.replace(" get", " absent")).is_err());
}

#[test]
fn local_take_transfers_ownership_and_rejects_consumed_paths_or_reordered_effects() {
    let text = "ciq-ir\n.function f\n.integer %0 1\n.local $0 %0\n.take %1 $0\n.return %1\n.end\n";
    let f = parse_function_ir(text).unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    assert!(
        f.lower()
            .to_text()
            .contains("lgetv 0x01\nnpush\nlputv 0x01")
    );
    let repeated = text.replace(".return %1", ".discard %1\n.read %2 $0\n.return %2");
    assert!(parse_function_ir(&repeated).is_err());
    let assigned = text.replace(".return %1", ".assign $0 %1\n.integer %2 0\n.return %2");
    assert!(parse_function_ir(&assigned).is_err());
    let branch = "ciq-ir\n.function f\n.block ^0\n.integer %0 1\n.local $0 %0\n.boolean %1 true\n.branch %1 ^1 ^2\n.block ^1\n.take %2 $0\n.discard %2\n.jump ^3\n.block ^2\n.jump ^3\n.block ^3\n.read %3 $0\n.return %3\n.end\n";
    assert!(parse_function_ir(branch).is_err());
    // Delayed emission must not move the destructive read across a write.
    let reordered = text.replace(".return %1", ".integer %2 2\n.assign $0 %2\n.return %1");
    assert!(parse_function_ir(&reordered).is_err());
}

#[test]
fn weak_reference_lifetimes_survive_saved_ir_and_complete_application_generation() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-weak-references/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".take "));
    assert!(text.contains("result=nullable-object"));
    let resumed = parse_unit_ir(&text).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [0x60; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
    let asm = unit.lower();
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
}
