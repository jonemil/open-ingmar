#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{
    DictionaryScalar, Operation,
    api::{ApiContext, ApiLevel},
    compile_function, compile_sources, compile_sources_with_api, parse_function_ir, parse_unit_ir,
    parse_unit_ir_with_api,
};

#[test]
fn source_value_ingress_rechecks_structural_signatures_after_ir_reload() {
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
            let ty =
                format!("Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.{interface}{null}>");
            for source in [
                format!("function f() as {ty} {{ return {{\"a\"=>new C()}}; }}"),
                format!("function f(d as {ty},c as C) {{ d[\"a\"]=c; }}"),
                format!("function f(d as {ty},c as C) {{ d.put(\"a\",c); }}"),
                format!(
                    "module M {{ function make() as Toybox.Lang.{interface} {{return new C();}} function f() as {ty} {{return {{\"a\"=>true ? M.make() : M.make(),\"b\"=>new C()}};}} }}"
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
                assert!(compile_sources(&[&source, &broken]).is_err());
            }
        }
    }
}

#[test]
fn dictionaries_preserve_invariance_missing_key_null_and_api_requirements() {
    for interface in ["Comparable", "Comparator"] {
        for null in ["", " or Null"] {
            let value = format!("Toybox.Lang.{interface}{null}");
            let ty = format!("Toybox.Lang.Dictionary<Toybox.Lang.String,{value}>");
            let source = format!(
                "function f(d as {ty}) as Toybox.Lang.Array<{value}> {{ return d.values(); }}"
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
            for lookup in ["d[\"a\"]", "d.get(\"a\")"] {
                let member = if interface == "Comparable" {
                    "compareTo(1)"
                } else {
                    "compare(1,2)"
                };
                let unsafe_read = format!("function f(d as {ty}) {{ return {lookup}.{member}; }}");
                assert!(checking::compile_function(&unsafe_read).is_err());
                let guarded = format!(
                    "function f(d as {ty}) {{ var v={lookup}; if(v!=null) {{return v.{member};}} else {{return 0;}} }}"
                );
                let function = compile_function(&guarded).unwrap();
                assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
            }
            for other in [
                "Toybox.Lang.Number",
                "Toybox.Lang.Object",
                "Toybox.Lang.Comparable or Null",
            ] {
                if value == other {
                    continue;
                }
                let source = format!(
                    "module M {{ function take(d as {ty}) {{return d.size();}} function f(d as Toybox.Lang.Dictionary<Toybox.Lang.String,{other}>) {{return M.take(d);}} }}"
                );
                assert!(checking::compile_sources(&[&source]).is_err());
            }
        }
    }
    for interface in ["Comparable", "Comparator"] {
        let source = format!(
            "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.{interface},Toybox.Lang.Number>) {{return d.size();}}"
        );
        let function = compile_function(&source).unwrap();
        assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
    }
}

#[test]
fn interface_constants_retain_scalar_tags_and_nullable_value_contracts() {
    let source = "function f() as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.Comparable> { return {\"number\"=>97,\"char\"=>'a',\"boolean\"=>true,\"long\"=>1l,\"float\"=>1.0,\"double\"=>1.0d,\"string\"=>\"a\"}; }";
    let function = compile_function(source).unwrap();
    assert!(function.operations().iter().any(|op| matches!(&op.operation,
        Operation::DictionaryConstant {entries,..} if matches!(entries[0].1,DictionaryScalar::Number(97)) && matches!(entries[1].1,DictionaryScalar::Char(97)))));
    assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
    for interface in ["Comparable", "Comparator"] {
        let nullable = format!(
            "function f() as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.{interface} or Null> {{ return {{\"a\"=>null}}; }}"
        );
        let function = compile_function(&nullable).unwrap();
        assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
        assert!(compile_function(&nullable.replace(" or Null", "")).is_err());
    }
    assert!(compile_function(&source.replace("Comparable", "Comparator")).is_err());
}

#[test]
fn interface_dictionary_formatting_propagates_constructor_callback_effects() {
    let source = "class Target {function initialize(){}} class Holder {var target as Target; function initialize(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.Comparable>,t as Target){var text=d.toString();self.target=t;}}";
    for ty in [
        "Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.Comparable>",
        "Toybox.Lang.Dictionary",
        "Toybox.Lang.Object",
    ] {
        let source = source.replace(
            "Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.Comparable>",
            ty,
        );
        let error = compile_sources(&[&source]).unwrap_err().to_string();
        assert!(error.contains("shutdown"), "{error}");
        let after = source.replace(
            "var text=d.toString();self.target=t;",
            "self.target=t;var text=d.toString();",
        );
        assert!(compile_sources(&[&after]).is_ok());
    }
    let wrapper = "module Formatter {function text(d as Toybox.Lang.Dictionary) as Toybox.Lang.String {return d.toString();}}";
    let erased = source.replace("d.toString()", "Formatter.text(d)");
    let error = compile_sources(&[&erased, wrapper])
        .unwrap_err()
        .to_string();
    assert!(error.contains("shutdown"), "{error}");
}
