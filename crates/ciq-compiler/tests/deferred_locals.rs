use ciq_compiler::{compile, compile_sources, parse_ir, parse_unit_ir};

#[test]
fn declarations_and_definite_assignment_preserve_saved_stages() {
    for source in [
        "function f() as Toybox.Lang.Number{var x; x=41; return x+1;}",
        "function f() as Toybox.Lang.Number{var unused; return 0;}",
        "function f() as Toybox.Lang.Number{var x,y=2,z; x=y; z=x+1; return z;}",
        "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Number{var x; if(flag){x=7;}else{x=8;} return x;}",
        "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Number{var x; if(flag){return 1;}else{x=8;} return x;}",
        "function f() as Toybox.Lang.Number{var x; do{x=7;}while(false); return x;}",
        "function f() as Toybox.Lang.Number{var x; for(x=0;x<3;x++){System.println(x);} return x;}",
        "function f() as Toybox.Lang.Number{for(var x, y=0;y<1;y++){x=1; System.println(x);} return 0;}",
        "function f(n as Toybox.Lang.Number) as Toybox.Lang.Number{var x; switch(n){case 0:x=1;break; default:x=2;} return x;}",
        "function f() as Toybox.Lang.String {var x; x=\"hello\"; return x;}",
        "function f() as Toybox.Lang.Number{var x; x=[1,2]; return x.size();}",
        "function f() as Toybox.Lang.Number{var x; x={1=>2}; return x.size();}",
        "function f() as Toybox.Lang.Number{var x; x=new[2]b; return x.size();}",
        "function f() as Toybox.Lang.Number{var x; x=:token; return x.toNumber();}",
    ] {
        let function = compile(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
        let asm = function.lower();
        assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    }
    let source = "class Token {function value() as Toybox.Lang.Number{return 41;}} function f() as Toybox.Lang.Number{var x; x=new Token(); return x.value();}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn reads_require_assignment_on_every_executable_path() {
    for source in [
        "function f() as Toybox.Lang.Number{var x; return x;}",
        "function f() as Toybox.Lang.Number{var x; return x==null;}",
        "function f() as Toybox.Lang.Number{var x; x=x+1; return x;}",
        "function f() as Toybox.Lang.Number{var x; x++; return 0;}",
        "function f() as Toybox.Lang.Number{var x; x+=1; return 0;}",
        "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Number{var x; if(flag){x=1;} return x;}",
        "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Number{var x; while(flag){x=1;} return x;}",
        "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Number{while(flag){var x; if(flag){x=1;} System.println(x);} return 0;}",
        "function f() as Toybox.Lang.Number{var x; x=1; x=\"different\"; return x;}",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
}

#[test]
fn saved_declarations_reject_forged_initialization_and_contracts() {
    for text in [
        "ciq-ir\n.function f\n.declare-local $0 integer\n.read %0 $0\n.return %0\n.end\n",
        "ciq-ir\n.function f\n.integer %0 1\n.assign $0 %0\n.read %1 $0\n.return %1\n.end\n",
        "ciq-ir\n.function f\n.declare-local $0 string\n.integer %0 1\n.assign $0 %0\n.integer %1 0\n.return %1\n.end\n",
        "ciq-ir\n.function f\n.declare-local $0 integer\n.declare-local $0 integer\n.integer %0 0\n.return %0\n.end\n",
        "ciq-ir\n.function f\n.declare-local $0 void\n.return-void\n.end\n",
    ] {
        assert!(parse_ir(text).is_err(), "accepted {text}");
    }
}

#[test]
fn declaration_contracts_cover_all_stored_domains_and_api_checks() {
    for (parameter, body) in [
        (
            "p as Toybox.Lang.Dictionary",
            "var x; x=p; return x.size();",
        ),
        ("p as Toybox.WatchUi.View", "var x; x=p; return 0;"),
        (
            "p as Toybox.Lang.Comparable",
            "var x; x=p; return x.compareTo(1);",
        ),
        (
            "p as Toybox.Lang.Method or Null",
            "var x; x=p; if(x!=null){x.invoke();} return 0;",
        ),
    ] {
        let function = compile(&format!(
            "function f({parameter}) as Toybox.Lang.Number{{{body}}}"
        ))
        .unwrap();
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    }
    let text =
        "ciq-ir\n.function f\n.declare-local $0 byte-array\n.integer %0 0\n.return %0\n.end\n";
    let function = parse_ir(text).unwrap();
    let low = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(2, 0, 0),
        ciq_compiler::api::ApiLevel(2, 0, 0),
    )
    .unwrap();
    assert!(function.validate_api(low).is_err());
    let maximum = (0..255).map(|i| format!("var v{i};")).collect::<String>();
    assert!(
        compile(&format!(
            "function f() as Toybox.Lang.Number{{{maximum}return 0;}}"
        ))
        .is_ok()
    );
    assert!(
        compile(&format!(
            "function f() as Toybox.Lang.Number{{{maximum}var extra;return 0;}}"
        ))
        .is_err()
    );
    let function = compile("function f() as Toybox.Lang.Number{var x;x=1;return x;}").unwrap();
    let code = function.lower().encode().unwrap();
    let ops = ciq_mc::decode(&code).unwrap();
    assert_eq!(
        ops.iter()
            .filter(|op| op.instruction.opcode() == ciq_mc::Opcode::Lputv)
            .count(),
        1
    );
}

#[test]
fn declarations_must_dominate_assignments_and_source_contracts_remain_exact() {
    let text = "ciq-ir\n.function f\n.block ^0\n.boolean %0 true\n.branch %0 ^1 ^2\n.block ^1\n.declare-local $0 integer\n.jump ^3\n.block ^2\n.jump ^3\n.block ^3\n.integer %1 1\n.assign $0 %1\n.integer %2 0\n.return %2\n.end\n";
    assert!(parse_ir(text).is_err());
    let unit = compile_sources(&[
        "class C{} class Other{} function f() as Toybox.Lang.Number{var c;c=new C();return 0;}",
    ])
    .unwrap();
    let text = unit.to_ir_text();
    let changed = text.replace(
        ".declare-local $0 source(C@Lang.Object)",
        ".declare-local $0 source(Other@Lang.Object)",
    );
    assert_ne!(text, changed);
    assert!(parse_unit_ir(&changed).is_err());
}
