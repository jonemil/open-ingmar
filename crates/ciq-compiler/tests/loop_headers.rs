#[path = "common/checking.rs"]
mod checking;

use ciq_compiler::{compile_sources, parse_unit_ir};

#[test]
fn dead_headers_keep_unit_checks_and_internal_builder_and_branch_proofs() {
    let source = r#"
        module M {
            function value() { return 1; }
            function f(n as Toybox.Lang.Number or Null) {
                var values=[1];
                for(var i=0;i<2;values=[M.value()]) { break; }
                if(n!=null) {
                    do {break;} while(false);
                    return n+1;
                }
                return values[0];
            }
            function g(n as Toybox.Lang.Number or Null) {
                var i=0;
                for(;i<2;i=(n!=null?n:0)) {break;}
                return i;
            }
        }
    "#;
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert_eq!(
        unit.lower(),
        ciq_assembly::Program::parse(&unit.lower().to_text()).unwrap()
    );
    for bad in [
        "function f(){for(var i=0;i<2;i=Missing.update()){break;}return 0;}",
        "module M {function update(n as Toybox.Lang.Boolean){return 1;} function f(){for(var i=0;i<2;i=M.update(1)){break;}return 0;}}",
        "function f(){do{break;}while(Missing.test());return 0;}",
        "module M {function test(n as Toybox.Lang.Boolean) as Toybox.Lang.Boolean {return n;} function f(){do{break;}while(M.test(1));return 0;}}",
        // Disconnected headers currently need their own nullable guard; enclosing
        // executable-path facts are deliberately not imported into dead roots.
        "function f(n as Toybox.Lang.Number or Null){var i=0;if(n!=null){for(;i<1;i=n+1){break;}}return i;}",
    ] {
        assert!(checking::compile_sources(&[bad]).is_err(), "{bad}");
    }
}

#[test]
fn loop_nesting_and_scope_diagnostics_remain_bounded() {
    for keyword in ["for(;;)", "do"] {
        let body = format!(
            "{}return;{}",
            format!("{keyword}{{").repeat(65),
            if keyword == "do" {
                "}while(false);".repeat(65)
            } else {
                "}".repeat(65)
            }
        );
        assert!(ciq_compiler::compile_function(&format!("function f(){{{body}}}")).is_err());
    }
    let source = "function f(){var i=0; for(i=0;i<2;++i){do{continue;}while(false);} return i;}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn header_lists_check_every_declaration_and_effect_including_dead_updates() {
    let source = "module M {function next(x){return x+1;} function f(){var a=[0];for(var i=M.next(0),j=M.next(i);i<3;i=M.next(i),a=[M.next(j)]){break;}return a[0];}}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    for source in [
        "function f(){for(var i=j,j=1;i<2;i++){}return 0;}",
        "function f(){for(var i=0,i=1;i<2;i++){}return 0;}",
        "function f(){for(var i=0,j=1;i<2;i++){}return j;}",
        "function f(){for(var i=0,j=1;i<2;i++,j-=false){break;}return 0;}",
        "function f(){for(var i=0,j=1;i<2;i++,j=Missing.next()){break;}return 0;}",
        "function f(){for(var i=0,var j=1;i<2;i++){}return 0;}",
        "function f(){for(var i=0,;i<2;i++){}return 0;}",
        "function f(){for(var i=0;i<2;i++,){}return 0;}",
        "function f(){for(var i=0;i<2;var j=1){}return 0;}",
        "function f(){var i=0;for(System.println(1);i<1;i++){}return 0;}",
        "function f(){var i=0;for(;i<1;System.println(i)){i++;}return 0;}",
        "function f(){var i=0;for(i=0,System.println(1);i<1;i++){}return 0;}",
        "function f(){var i=0;for(;i<1;i++,System.println(i)){}return 0;}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let declarations = (0..256)
        .map(|n| format!("n{n}=0"))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!("function f(){{for(var {declarations};false;){{}}return 0;}}");
    let error = ciq_compiler::compile_function(&source).unwrap_err();
    assert!(error.message.contains("255-local limit"), "{error:?}");
}
