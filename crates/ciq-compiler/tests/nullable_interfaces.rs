use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    compile_function, compile_sources, parse_function_ir, parse_unit_ir,
};

#[test]
fn nullable_signatures_guards_and_edges_survive_saved_ir() {
    let current = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap();
    let low = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(4, 2, 0)).unwrap();
    for interface in ["Comparable", "Comparator"] {
        let source = format!(
            "function f(c as Toybox.Lang.{interface} or Null) as Toybox.Lang.{interface} or Null {{ return c; }}"
        );
        let function = compile_function(&source).unwrap();
        function.validate_api(current).unwrap();
        assert!(function.validate_api(low).is_err());
        assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
        let text = function.to_ir_text();
        let changed = text.replace(
            &format!("result=nullable-{}", interface.to_lowercase()),
            "result=boolean",
        );
        assert_ne!(text, changed);
        assert!(parse_function_ir(&changed).is_err());
    }
    for body in [
        "if(c!=null) { return c.compareTo(1); } else { return 0; }",
        "if(null==c) { return 0; } return c.compareTo(1);",
        "var local=c; while(local!=null) { var n=local.compareTo(1); local=null; } return 0;",
        "return c!=null ? c.compareTo(1) : 0;",
    ] {
        let source = format!("function f(c as Toybox.Lang.Comparable or Null) {{ {body} }}");
        let function = compile_function(&source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
    }
    for ty in [
        "Number", "Long", "Float", "Double", "Boolean", "Char", "String",
    ] {
        let source = format!(
            "function f(c as Toybox.Lang.{ty} or Null) as Toybox.Lang.Comparable or Null {{ return c; }}"
        );
        let function = compile_function(&source).unwrap();
        assert_eq!(
            function,
            parse_function_ir(&function.to_ir_text()).unwrap(),
            "{ty}"
        );
    }
}

#[test]
fn nullable_receivers_require_fresh_storage_proofs() {
    for body in [
        "return c.compareTo(1);",
        "if(c==null) { return c.compareTo(1); } else { return 0; }",
        "var copy=c; if(c!=null) { return copy.compareTo(1); } else { return 0; }",
        "var guard=c!=null; if(guard) { return c.compareTo(1); } else { return 0; }",
        "var local=c; if(local!=null) { local=null; return local.compareTo(1); } else { return 0; }",
        "var local=c; var old=local; if(local!=null) { return old.compareTo(1); } else { return 0; }",
    ] {
        let source = format!("function f(c as Toybox.Lang.Comparable or Null) {{ {body} }}");
        assert!(compile_function(&source).is_err(), "{source}");
    }
    for source in [
        "function f(c as Toybox.Lang.Comparator or Null) { return c.compare(1,2); }",
        "function f(c as Toybox.Lang.Comparable or Null) as Toybox.Lang.Comparable { return c; }",
        "function f(c as Toybox.Lang.Comparator or Null) as Toybox.Lang.Comparable or Null { return c; }",
        "function f(c as Toybox.Lang.Comparable or Null) { var a=[1]; a.sort(c); }",
        "function f(o as Toybox.Lang.Object) { return o instanceof Toybox.Lang.Comparator; }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let function = compile_function(
        "function f(c as Toybox.Lang.Comparator or Null) { var a=[1,2]; a.sort(c); }",
    )
    .unwrap();
    assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
}

#[test]
fn source_assignments_require_unit_member_proofs_after_text_reload() {
    for (interface, parameters, member) in [
        ("Comparable", "a as Toybox.Lang.Object", "compareTo"),
        (
            "Comparator",
            "a as Toybox.Lang.Object,b as Toybox.Lang.Object",
            "compare",
        ),
    ] {
        let class = format!(
            "class C {{ function initialize() {{}} function {member}({parameters}) as Toybox.Lang.Number {{ return 0; }} }}"
        );
        for null in ["", " or Null"] {
            let source = format!(
                "function f(initial as Toybox.Lang.{interface}{null}) as Toybox.Lang.{interface}{null} {{ var local=initial; local=new C(); return local; }}"
            );
            let unit = compile_sources(&[&source, &class]).unwrap();
            let text = unit.to_ir_text();
            assert_eq!(unit, parse_unit_ir(&text).unwrap());
            let changed = text.replace("parameter-types=object", "parameter-types=integer");
            assert_ne!(text, changed);
            assert!(parse_unit_ir(&changed).is_err(), "{interface}{null}");
            let invalid = class.replace("a as Toybox.Lang.Object", "a as Toybox.Lang.Number");
            assert!(compile_sources(&[&source, &invalid]).is_err());
        }
        let source = format!(
            "module M {{ function take(c as Toybox.Lang.{interface} or Null) {{ return 0; }} }} function f() {{ return M.take(new C()); }}"
        );
        let unit = compile_sources(&[&source, &class]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
}
