use ciq_compiler::{compile_function, compile_sources, parse_function_ir, parse_unit_ir};

#[test]
fn native_union_signatures_narrow_and_preserve_saved_stages() {
    for source in [
        "function f(x as Toybox.Lang.Number or Toybox.Lang.String) as Toybox.Lang.String{return x.toString();}",
        "function f(x as Toybox.Lang.Number or Toybox.Lang.String) as Toybox.Lang.Number{if(x instanceof Toybox.Lang.Number){return x+1;}else{return x.length();}}",
        "function f(x as Null or Toybox.Lang.String or Toybox.Lang.Number) as Toybox.Lang.Number{if(x==null){return 0;}if(x instanceof Toybox.Lang.String){return x.length();}return x+1;}",
        "function f(x as Toybox.Lang.Array<Toybox.Lang.Number> or Toybox.Lang.String) as Toybox.Lang.Number{if(x instanceof Toybox.Lang.Array){return x[0];}return x.length();}",
        "function f(x as Toybox.Lang.Number or Toybox.Lang.String,b as Toybox.Lang.Boolean) as Toybox.Lang.Number or Toybox.Lang.String{var y=x;if(b){y=\"hi\";}return y;}",
        "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.ByteArray or Toybox.Lang.Number{return b?[]b:42;}",
        "function f(x as Toybox.Lang.Number or Toybox.Lang.String) as Toybox.Lang.Number or Toybox.Lang.String{return x+1;}",
    ] {
        let f = compile_function(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        assert_eq!(
            f.lower(),
            ciq_assembly::Program::parse(&f.assembly()).unwrap()
        );
    }
}

#[test]
fn normalization_is_order_independent_and_calls_check_every_alternative() {
    let a=compile_function("function f(x as Toybox.Lang.String or Toybox.Lang.Number or Toybox.Lang.String) as Toybox.Lang.Number or Toybox.Lang.String{return x;}").unwrap();
    let b=compile_function("function f(x as Toybox.Lang.Number or Toybox.Lang.String) as Toybox.Lang.String or Toybox.Lang.Number{return x;}").unwrap();
    assert_eq!(a.parameter_types(), b.parameter_types());
    assert_eq!(a.return_type(), b.return_type());
    let source = "module M{function echo(x as Toybox.Lang.Number or Toybox.Lang.String) as Toybox.Lang.Number or Toybox.Lang.String{return x;}} function f(b as Toybox.Lang.Boolean){return M.echo(b?1:\"x\");}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    for source in [
        "function f(x as Toybox.Lang.Number or Toybox.Lang.String){return x.length();}",
        "function f(x as Toybox.Lang.Number or Toybox.Lang.String){return x-1;}",
        "function f(x as Toybox.Lang.Number or Toybox.Lang.String) as Toybox.Lang.Number{return x;}",
        "function f(x as Toybox.Lang.Number or Toybox.Lang.String){if(x instanceof Toybox.Lang.Number){x=\"hi\";return x-1;}return 0;}",
        "function f(x as Toybox.Lang.Number or Void){return x;}",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    let source = "module M{function use(x as Toybox.Lang.Number or Toybox.Lang.String){return x;}}function f(){return M.use(true);}";
    assert!(compile_sources(&[source]).is_err());
}

#[test]
fn container_unions_keep_each_content_contract_and_require_non_null_receivers() {
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number> or Toybox.Lang.Array<Toybox.Lang.Long>) as Toybox.Lang.Number or Toybox.Lang.Long{return a[0];}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number> or Toybox.Lang.ByteArray){a[0]=7;return a[0];}",
        "function f(a as Toybox.Lang.Dictionary or Null){if(a==null){return 0;}return a.size();}",
        "function f(s as Toybox.Lang.Symbol or Null) as Toybox.Lang.Boolean{return s==:name;}",
    ] {
        let f = compile_function(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        let unit = compile_sources(&[source]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number> or Toybox.Lang.Array<Toybox.Lang.Long>){a[0]=1;return 0;}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number> or Null){return a[0];}",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
}

#[test]
fn saved_union_contracts_reject_forged_reads_and_recheck_availability() {
    let text = "ciq-ir\n.function f result=union(integer|string)\n.declare-local $0 union(integer|string)\n.integer %0 1\n.assign $0 %0\n.read %1 $0\n.return %1\n.end\n";
    let f = parse_function_ir(text).unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    for changed in [
        text.replace(".integer %0 1", ".boolean %0 true"),
        text.replace("union(integer|string)", "union(integer|void)"),
        text.replace("union(integer|string)", "union(one-view|string)"),
        text.replace(
            "union(integer|string)",
            "union(integer|source(C@Lang.Object))",
        ),
    ] {
        assert!(parse_function_ir(&changed).is_err());
    }
    let f = compile_function(
        "function f(x as Toybox.Lang.ByteArray or Toybox.Lang.String){return x.toString();}",
    )
    .unwrap();
    let low = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(1, 0, 0),
        ciq_compiler::api::ApiLevel(1, 0, 0),
    )
    .unwrap();
    assert!(f.validate_api(low).is_err());
    assert!(ciq_compiler::parse_function_ir_with_api(&f.to_ir_text(), low).is_err());
    let valid = "function f(x as Toybox.Lang.Number or Toybox.Lang.String) as Toybox.Lang.Number{if(!(x instanceof Toybox.Lang.Number)){return x.length();}return x+1;}";
    let f = compile_function(valid).unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
}
