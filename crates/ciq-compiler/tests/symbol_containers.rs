#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{
    Operation,
    api::{ApiContext, ApiLevel},
    compile_function, compile_function_with_api, compile_sources, parse_function_ir, parse_unit_ir,
    symbol_bindings,
};

#[test]
fn symbol_literals_use_runtime_construction_and_explicit_name_binding() {
    for (source, builder) in [
        (
            "function f() as Toybox.Lang.Array<Toybox.Lang.Symbol> {return [:alpha,:size,:alpha];}",
            ".array-begin",
        ),
        (
            "function f() as Toybox.Lang.Dictionary<Toybox.Lang.Symbol,Toybox.Lang.Symbol> {return {:alpha=>:size};}",
            ".dictionary-begin",
        ),
    ] {
        let function = compile_function(source).unwrap();
        let text = function.to_ir_text();
        assert!(text.contains(builder));
        assert!(!text.contains("-constant"));
        assert_eq!(function, parse_function_ir(&text).unwrap());
        let assembly = function.lower();
        assert_eq!(
            assembly,
            ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
        );
        for id in [0x01020304u32, 0x10203040] {
            let symbols = ciq_assembly::Program::parse(&format!(
                ".symbol a \"alpha\" id=0x{id:08x}\n.symbol s \"size\" id=0x008000ae"
            ))
            .unwrap();
            let bindings = symbol_bindings(&assembly, &symbols).unwrap();
            let code = assembly
                .encode_with_literals(&bindings, &Default::default(), &Default::default())
                .unwrap();
            let mut pushed = vec![0x27];
            pushed.extend(id.to_be_bytes());
            assert!(code.windows(5).any(|w| w == pushed));
            assert!(code.windows(5).any(|w| w == [0x27, 0, 0x80, 0, 0xae]));
            ciq_mc::decode(&code).unwrap();
        }
        let missing = ciq_assembly::Program::parse(".symbol a \"alpha\" id=0x1").unwrap();
        assert!(symbol_bindings(&assembly, &missing).is_err());
        let line = text
            .lines()
            .find(|line| line.starts_with(".symbol "))
            .unwrap();
        let id = line.split_whitespace().nth(1).unwrap();
        let edited = text.replace(line, &format!(".integer {id} 1"));
        assert!(parse_function_ir(&edited).is_err());
    }
}

#[test]
fn container_signatures_keep_symbol_nullability_and_api_levels() {
    let initial = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap();
    for ty in [
        "Toybox.Lang.Array<Toybox.Lang.Symbol>",
        "Toybox.Lang.Array<Toybox.Lang.Symbol or Null>",
        "Toybox.Lang.Dictionary<Toybox.Lang.Symbol,Toybox.Lang.Symbol>",
        "Toybox.Lang.Dictionary<Toybox.Lang.Symbol,Toybox.Lang.Symbol or Null>",
        "Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.Symbol or Null>",
    ] {
        let source = format!("function f(value as {ty}) as {ty} {{return value;}}");
        let function = compile_function_with_api(&source, initial).unwrap();
        assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
    }
    for (method, element) in [("keys", "Symbol"), ("values", "Symbol or Null")] {
        let source = format!(
            "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.Symbol,Toybox.Lang.Symbol or Null>) as Toybox.Lang.Array<Toybox.Lang.{element}> {{return d.{method}();}}"
        );
        let function = compile_function(&source).unwrap();
        assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
    }
    for source in [
        "function f() as Toybox.Lang.Array<Toybox.Lang.Symbol or Null> {return [null];}",
        "function f() as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.Symbol or Null> {return {\"a\"=>null};}",
    ] {
        let function = compile_function(source).unwrap();
        assert!(function.operations().iter().any(|op| matches!(
            op.operation,
            Operation::ArrayConstant { .. } | Operation::DictionaryConstant { .. }
        )));
        assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
    }
    let inferred = "function f() as Toybox.Lang.Dictionary<Toybox.Lang.Symbol,Toybox.Lang.Symbol> {var d={:alpha=>:beta};return d;}";
    assert!(compile_function(inferred).is_ok());
}

#[test]
fn fresh_guards_protect_symbol_key_arguments_and_indexed_snapshots() {
    let dictionary = "Toybox.Lang.Dictionary<Toybox.Lang.Symbol,Toybox.Lang.Symbol or Null>";
    for operation in [
        "var value=d[k];",
        "d[k]=:alpha;",
        "var value=d.get(k);",
        "d.put(k,:alpha);",
        "var value=d.hasKey(k);",
        "d.remove(k);",
    ] {
        let source = format!(
            "function f(d as {dictionary},k as Toybox.Lang.Symbol or Null) {{{operation}}}"
        );
        assert!(checking::compile_function(&source).is_err(), "{source}");
        let guarded = format!(
            "function f(d as {dictionary},k as Toybox.Lang.Symbol or Null) {{if(k!=null){{{operation}}}}}"
        );
        let function = compile_function(&guarded).unwrap_or_else(|e| panic!("{guarded}: {e}"));
        assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
    }
    for (receiver, read) in [
        ("Toybox.Lang.Array<Toybox.Lang.Symbol or Null>", "value[0]"),
        (dictionary, "value[:alpha]"),
    ] {
        let source = format!(
            "function f(value as {receiver}) as Toybox.Lang.String {{var item={read};if(item!=null){{return item.toString();}}return \"missing\";}}"
        );
        let function = compile_function(&source).unwrap();
        assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
        let stale = format!(
            "function f(value as {receiver}) as Toybox.Lang.String {{if({read}!=null){{return {read}.toString();}}return \"missing\";}}"
        );
        assert!(checking::compile_function(&stale).is_err());
    }
}

#[test]
fn symbol_containers_do_not_widen_or_invent_scalar_payloads() {
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Symbol>){a.add(null);}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Symbol>){a[0]=1;}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Symbol>){a.sort(null);}",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Comparable>{return [:alpha];}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Symbol>) as Toybox.Lang.Array<Toybox.Lang.Symbol or Null>{return a;}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.Symbol,Toybox.Lang.Symbol>){d[:alpha]=null;}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.Symbol,Toybox.Lang.Symbol>){d.put(1,:alpha);}",
        "function f(d as Toybox.Lang.Dictionary<Toybox.Lang.Symbol or Null,Toybox.Lang.Number>){return d.size();}",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    for operation in [
        ".array-constant %0 symbol 0x00000001",
        ".array-constant %0 nullable-symbol 0x00000001",
        ".dictionary-constant %0 dictionary(symbol=>integer) number 0x00000001 number 0x00000002",
    ] {
        assert!(
            parse_function_ir(&format!(
                "ciq-ir\n.function f\n{operation}\n.return %0\n.end\n"
            ))
            .is_err()
        );
    }
    let source = "module M {function array() as Toybox.Lang.Array<Toybox.Lang.Symbol>{return [:alpha];} function dictionary() as Toybox.Lang.Dictionary<Toybox.Lang.Symbol,Toybox.Lang.Symbol>{return {:alpha=>:beta};} function run(){var a=M.array();var d=M.dictionary();a.add(:beta);d.put(:beta,a[0]);}}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}
