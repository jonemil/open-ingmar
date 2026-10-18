use ciq_compiler::{
    Operation,
    api::{ApiContext, ApiLevel},
    compile, compile_sources, compile_sources_with_api, parse_ir, parse_unit_ir,
    parse_unit_ir_with_api,
};

#[test]
fn every_source_element_ingress_rechecks_the_implementation() {
    for (interface, member, parameters) in [
        ("Comparable", "compareTo", "a as Toybox.Lang.Object"),
        (
            "Comparator",
            "compare",
            "a as Toybox.Lang.Object,b as Toybox.Lang.Object",
        ),
    ] {
        let class = format!(
            "class C {{ function initialize() {{}} function {member}({parameters}) as Toybox.Lang.Number {{ return 0; }} }}"
        );
        for null in ["", " or Null"] {
            let ty = format!("Toybox.Lang.{interface}{null}");
            let cases = [
                format!("function f() as Toybox.Lang.Array<{ty}> {{ return [new C()]; }}"),
                format!(
                    "module M {{ function create() as Toybox.Lang.{interface} {{ return new C(); }} function f() as Toybox.Lang.Array<{ty}> {{ return [true ? M.create() : M.create(),new C()]; }} }}"
                ),
                format!("function f(a as Toybox.Lang.Array<{ty}>, c as C) {{ a[0]=c; }}"),
                format!("function f(a as Toybox.Lang.Array<{ty}>, c as C) {{ a.add(c); }}"),
                format!(
                    "function f(a as Toybox.Lang.Array<{ty}>, c as C) {{ return a.indexOf(c); }}"
                ),
                format!(
                    "function f(a as Toybox.Lang.Array<{ty}>, c as C) as Toybox.Lang.Boolean {{ return a.remove(c); }}"
                ),
                format!(
                    "function f(a as Toybox.Lang.Array<{ty}>, c as C) as Toybox.Lang.Boolean {{ return a.removeAll(c); }}"
                ),
            ];
            for source in cases {
                let unit =
                    compile_sources(&[&source, &class]).unwrap_or_else(|e| panic!("{source}: {e}"));
                let text = unit.to_ir_text();
                let resumed = parse_unit_ir(&text).unwrap();
                assert_eq!(unit, resumed);
                assert_eq!(unit.lower(), resumed.lower());
                let changed = text.replace("parameter-types=object", "parameter-types=integer");
                assert_ne!(text, changed);
                assert!(parse_unit_ir(&changed).is_err(), "{source}");
                let invalid = class.replace("a as Toybox.Lang.Object", "a as Toybox.Lang.Number");
                assert!(compile_sources(&[&source, &invalid]).is_err(), "{source}");
            }
        }
    }
}

#[test]
fn array_contracts_are_invariant_and_reads_preserve_nullable_interfaces() {
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Array<Toybox.Lang.Comparable> { return a; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Comparable>) as Toybox.Lang.Array<Toybox.Lang.Comparable or Null> { return a; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Comparable>,b as Toybox.Lang.Array<Toybox.Lang.Number>) { a.addAll(b); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Comparable>) { a.add(null); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Comparable>,o as Toybox.Lang.Object) { a[0]=o; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Comparator>) { a[0]=1; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Comparable or Null>) { return a[0].compareTo(1); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Comparable or Null>) { if(a[0]!=null) { return a[0].compareTo(1); } else { return 0; } }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Comparable>) { return a[0].value(); }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let source = "function f(a as Toybox.Lang.Array<Toybox.Lang.Comparable or Null>) { var value=a[0]; if(value!=null) { return value.compareTo(1); } else { return 0; } }";
    let function = compile(source).unwrap();
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    for interface in ["Comparable", "Comparator"] {
        for null in ["", " or Null"] {
            let ty = format!("Toybox.Lang.Array<Toybox.Lang.{interface}{null}>");
            let source =
                format!("function f(a as {ty}) as {ty} {{ return a.reverse().slice(0,null); }}");
            let unit = compile_sources(&[&source]).unwrap();
            let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap();
            unit.validate_api(context).unwrap();
            let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap();
            assert_eq!(unit, resumed);
            let low = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(4, 2, 0)).unwrap();
            assert!(compile_sources_with_api(&[&source], low).is_err());
            assert!(parse_unit_ir_with_api(&unit.to_ir_text(), low).is_err());
            let sort = format!("function f(a as {ty}) {{ a.sort(null); }}");
            assert!(compile(&sort).is_err());
        }
    }
}

#[test]
fn interface_literals_keep_runtime_scalar_tags_and_private_initialization() {
    let source = "function f() as Toybox.Lang.Array<Toybox.Lang.Comparable> { return [1,'a',true,1l,1.0,1.0d,\"x\"]; }";
    let function = compile(source).unwrap();
    assert!(
        !function
            .operations()
            .iter()
            .any(|op| matches!(op.operation, Operation::ArrayConstant { .. }))
    );
    assert!(
        function
            .operations()
            .iter()
            .any(|op| matches!(op.operation, Operation::Char { .. }))
    );
    assert!(
        function
            .operations()
            .iter()
            .any(|op| matches!(op.operation, Operation::Integer { .. }))
    );
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    let target = "class Target { function initialize() {} }";
    for operation in [
        "var n=a.indexOf(c);",
        "var n=a.remove(c);",
        "var n=a.removeAll(c);",
        "var n=a.toString();",
    ] {
        let source = format!(
            "class Holder {{ var target as Target; function initialize(a as Toybox.Lang.Array<Toybox.Lang.Comparable>,c as Toybox.Lang.Comparable,t as Target) {{ {operation} self.target=t; }} }}"
        );
        let error = compile_sources(&[&source, target]).unwrap_err().to_string();
        assert!(error.contains("shutdown"), "{error}");
        let after = source.replace(
            &format!("{operation} self.target=t;"),
            &format!("self.target=t; {operation}"),
        );
        assert!(compile_sources(&[&after, target]).is_ok(), "{after}");
    }
}
