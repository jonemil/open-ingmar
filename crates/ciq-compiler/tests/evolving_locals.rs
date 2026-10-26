#[path = "common/checking.rs"]
mod checking;

use ciq_compiler::{compile_function, compile_sources, parse_function_ir, parse_unit_ir};

#[test]
fn changing_local_types_and_joins_preserve_saved_stages() {
    for source in [
        "function f(){var x=1;System.println(x-1);x=\"hi\";return x.length();}",
        "function f(){var x; x=1;x=\"hi\";return x.length();}",
        "function f(b as Toybox.Lang.Boolean){var x=\"before\";if(b){x=1;}else{x=2;}return x+1;}",
        "function f(b as Toybox.Lang.Boolean){var x=1;if(b){x=\"hi\";}if(x instanceof Toybox.Lang.String){return x.length();}return 0;}",
        "function f(){var x=1;while(x instanceof Toybox.Lang.Number){System.println(x-1);x=\"done\";}return 0;}",
        "function f(b as Toybox.Lang.Boolean){var x=1;if(b){x=null;}if(x!=null){return x+1;}return 0;}",
        "function f(){var x=1;var old=x;x=\"hi\";return old+1;}",
        "function f(){var x=[1];var shared=x;x=\"hi\";return shared[0]+x.length();}",
        "function f(){var x=null;x=41;return x+1;}",
        "function f(){var x=1;do{x=\"after\";}while(false);return x.length();}",
        "function f() as Toybox.Lang.String{var x='a';x+='b';return x;}",
        "function f() as Toybox.Lang.Char{var x=1;x+='b';return x;}",
        "function f() as Toybox.Lang.Long{var x=1;x+=2l;return x;}",
    ] {
        let f = compile_function(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        assert_eq!(
            f.lower(),
            ciq_assembly::Program::parse(&f.assembly()).unwrap()
        );
    }
    let source = "class A{function a(){return 1;}} class B{function b(){return 2;}} function f(){var x=new A();var old=x;x=new B();return old.a()+x.b();}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn flow_read_metadata_is_checked_not_trusted() {
    let f = compile_function("function f(){var x=1;x=\"hi\";return x.length();}").unwrap();
    let text = f.to_ir_text();
    assert!(text.contains(".variable"));
    assert!(text.contains(".read-variable"));
    let changed = text.replace(" string", " integer");
    assert_ne!(text, changed);
    assert!(parse_function_ir(&changed).is_err());
    for source in [
        "function f(b as Toybox.Lang.Boolean){var x=1;if(b){x=\"hi\";}return x-1;}",
        "function f(b as Toybox.Lang.Boolean){var x=1;while(b){x=\"hi\";System.println(x-1);}return 0;}",
        "function f(){var x=1;x=\"hi\";return x-1;}",
    ] {
        assert!(
            checking::compile_function(source).is_err(),
            "accepted {source}"
        );
    }
}

#[test]
fn forged_read_claims_cannot_create_a_type_or_skip_initialization() {
    for body in [
        ".integer %0 1\n.variable $0 %0\n.string %1 \"hi\"\n.assign $0 %1\n.read-variable %2 $0 integer\n.return %2",
        ".variable $0\n.read-variable %0 $0 integer\n.return %0",
        ".integer %0 1\n.variable $0 %0\n.read %1 $0\n.return %1",
        ".integer %0 1\n.variable $0 %0\n.take %1 $0\n.return %1",
        ".integer %0 1\n.variable $0 %0 %0\n.return %0",
        ".variable $0 %9\n.integer %0 0\n.return %0",
    ] {
        let text = format!("ciq-ir\n.function f\n{body}\n.end\n");
        assert!(parse_function_ir(&text).is_err(), "accepted {text}");
    }
    for source in [
        "function f(b as Toybox.Lang.Boolean){var x=1;if(b){x=2l;}return x.toNumber();}",
        "function f(){var x=1;x=\"done\";x=2;x++;return x;}",
        "function f(){var x=1;x=null;x=\"done\";return x.length();}",
        "function f(b as Toybox.Lang.Boolean){var x=1;if(b){x=\"hi\";}else{return 0;}return x.length();}",
    ] {
        let f = compile_function(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
}
