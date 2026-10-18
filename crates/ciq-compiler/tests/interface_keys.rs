use ciq_compiler::{
    Operation,
    api::{ApiContext, ApiLevel},
    compile, compile_sources, compile_sources_with_api, parse_ir, parse_unit_ir,
    parse_unit_ir_with_api,
};

#[test]
fn every_key_ingress_rechecks_source_signatures_after_ir_reload() {
    for (interface, member, parameters) in [
        ("Comparable", "compareTo", "a as Toybox.Lang.Object"),
        (
            "Comparator",
            "compare",
            "a as Toybox.Lang.Object,b as Toybox.Lang.Object",
        ),
    ] {
        let class = format!(
            "class C {{function initialize(){{}} function {member}({parameters}) as Toybox.Lang.Number {{return 0;}}}}"
        );
        let ty = format!("Toybox.Lang.Dictionary<Toybox.Lang.{interface},Toybox.Lang.Number>");
        for source in [
            format!("function f(c as C) as {ty} {{return {{c=>1,new C()=>2}};}}"),
            format!("function f(d as {ty},c as C) {{d[c]=1;}}"),
            format!("function f(d as {ty},c as C) as Toybox.Lang.Number or Null {{return d[c];}}"),
            format!("function f(d as {ty},c as C) {{d.put(c,1);}}"),
            format!(
                "function f(d as {ty},c as C) as Toybox.Lang.Number or Null {{return d.get(c);}}"
            ),
            format!("function f(d as {ty},c as C) as Toybox.Lang.Boolean {{return d.hasKey(c);}}"),
            format!("function f(d as {ty},c as C) {{d.remove(c);}}"),
            format!(
                "module M {{function key() as Toybox.Lang.{interface} {{return new C();}} function f() as {ty} {{return {{(true ? M.key() : M.key())=>1,new C()=>2}};}}}}"
            ),
        ] {
            let unit =
                compile_sources(&[&source, &class]).unwrap_or_else(|e| panic!("{source}: {e}"));
            let text = unit.to_ir_text();
            assert_eq!(unit, parse_unit_ir(&text).unwrap());
            let edited = text.replace("parameter-types=object", "parameter-types=integer");
            assert_ne!(text, edited);
            assert!(parse_unit_ir(&edited).is_err(), "{source}");
            let broken = class.replace("a as Toybox.Lang.Object", "a as Toybox.Lang.Number");
            assert!(compile_sources(&[&source, &broken]).is_err(), "{source}");
        }
    }
}

#[test]
fn invariant_keys_require_api_five_and_non_null_witnesses() {
    for interface in ["Comparable", "Comparator"] {
        let ty = format!("Toybox.Lang.Dictionary<Toybox.Lang.{interface},Toybox.Lang.Number>");
        let source = format!(
            "function f(d as {ty}) as Toybox.Lang.Array<Toybox.Lang.{interface}> {{return d.keys();}}"
        );
        let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap();
        let unit = compile_sources_with_api(&[&source], context).unwrap();
        assert_eq!(
            unit,
            parse_unit_ir_with_api(&unit.to_ir_text(), context).unwrap()
        );
        let low = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(4, 2, 0)).unwrap();
        assert!(compile_sources_with_api(&[&source], low).is_err());
        assert!(parse_unit_ir_with_api(&unit.to_ir_text(), low).is_err());
        for operation in ["d[k]", "d.get(k)", "d.hasKey(k)"] {
            let source = format!(
                "function f(d as {ty},k as Toybox.Lang.{interface} or Null) {{var result={operation};}}"
            );
            assert!(compile(&source).is_err());
            let guarded = source.replace(
                &format!("var result={operation};"),
                &format!("if(k!=null){{var result={operation};}}"),
            );
            let function = compile(&guarded).unwrap();
            assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
        }
        for key in [
            "Toybox.Lang.Number",
            "Toybox.Lang.Object",
            "Toybox.Lang.Comparator",
            "Toybox.Lang.Comparable",
        ] {
            if key == format!("Toybox.Lang.{interface}") {
                continue;
            }
            let source = format!(
                "module M {{function take(d as {ty}){{return d.size();}} function f(d as Toybox.Lang.Dictionary<{key},Toybox.Lang.Number>) {{return M.take(d);}}}}"
            );
            assert!(compile_sources(&[&source]).is_err());
        }
        assert!(
            compile(&source.replace(&format!("{interface},"), &format!("{interface} or Null,")))
                .is_err()
        );
    }
}

#[test]
fn interface_key_literals_use_runtime_insertion_and_reject_ambiguous_constants() {
    let source = "function f() as Toybox.Lang.Dictionary<Toybox.Lang.Comparable,Toybox.Lang.Number> {return {1=>10,2=>20};}";
    let function = compile(source).unwrap();
    assert!(
        function
            .operations()
            .iter()
            .any(|op| matches!(op.operation, Operation::DictionaryBegin { .. }))
    );
    assert!(
        !function
            .operations()
            .iter()
            .any(|op| matches!(op.operation, Operation::DictionaryConstant { .. }))
    );
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    let mixed = source.replace("1=>10,2=>20", "97=>1,'a'=>2");
    assert!(
        compile(&mixed)
            .unwrap_err()
            .to_string()
            .contains("runtime insertion")
    );
    let scalar = "function f() as Toybox.Lang.Dictionary<Toybox.Lang.Number,Toybox.Lang.Number> {return {1=>1};}";
    let text = compile(scalar).unwrap().to_ir_text();
    let edited = text.replace(
        "dictionary(integer=>integer)",
        "dictionary(comparable=>integer)",
    );
    assert_ne!(text, edited);
    assert!(parse_ir(&edited).is_err());
}

#[test]
fn key_callbacks_cannot_run_before_reference_fields_are_initialized() {
    let ty = "Toybox.Lang.Dictionary<Toybox.Lang.Comparable,Toybox.Lang.Number>";
    let factory = format!(
        "module Factory {{function make(k as Toybox.Lang.Comparable) as {ty} {{return {{k=>1}};}}}}"
    );
    for operation in [
        "var n=d[k];",
        "d[k]=1;",
        "var n=d.get(k);",
        "d.put(k,1);",
        "var n=d.hasKey(k);",
        "d.remove(k);",
        "var n=d.toString();",
        "var n=Factory.make(k);",
    ] {
        let source = format!(
            "class Target {{function initialize(){{}}}} class Holder {{var target as Target; function initialize(d as {ty},k as Toybox.Lang.Comparable,t as Target){{{operation} self.target=t;}}}}"
        );
        let error = compile_sources(&[&source, &factory])
            .unwrap_err()
            .to_string();
        assert!(error.contains("shutdown"), "{operation}: {error}");
        let after = source.replace(
            &format!("{operation} self.target=t;"),
            &format!("self.target=t; {operation}"),
        );
        assert!(compile_sources(&[&after, &factory]).is_ok(), "{after}");
    }
}

#[test]
fn object_key_overrides_preserve_runtime_signatures_even_after_erasure() {
    let valid = "class C {function initialize(){} function hashCode() as Toybox.Lang.Number{return 1;} function equals(other as Toybox.Lang.Object or Null) as Toybox.Lang.Boolean{return false;}}";
    let unit = compile_sources(&[valid]).unwrap();
    let text = unit.to_ir_text();
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    for edited in [
        text.replace("parameter-types=nullable-object", "parameter-types=integer"),
        text.replace("parameter-types=nullable-object", "parameter-types=object"),
    ] {
        assert_ne!(text, edited);
        assert!(parse_unit_ir(&edited).is_err());
    }
    for broken in [
        valid.replace("hashCode()", "hashCode(extra as Toybox.Lang.Number)"),
        valid.replace(
            "other as Toybox.Lang.Object or Null",
            "other as Toybox.Lang.Number",
        ),
        valid.replace("return 1;", "return true;"),
    ] {
        assert!(compile_sources(&[&broken]).is_err());
    }
}
