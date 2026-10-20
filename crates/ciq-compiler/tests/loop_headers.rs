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
        "function f(){for(var i=0;i<2;Missing.update()){break;}return 0;}",
        "module M {function update(n as Toybox.Lang.Boolean){} function f(){for(var i=0;i<2;M.update(1)){break;}return 0;}}",
        "function f(){do{break;}while(Missing.test());return 0;}",
        "module M {function test(n as Toybox.Lang.Boolean) as Toybox.Lang.Boolean {return n;} function f(){do{break;}while(M.test(1));return 0;}}",
        // Disconnected headers currently need their own nullable guard; enclosing
        // executable-path facts are deliberately not imported into dead roots.
        "function f(n as Toybox.Lang.Number or Null){var i=0;if(n!=null){for(;i<1;i=n+1){break;}}return i;}",
    ] {
        assert!(compile_sources(&[bad]).is_err(), "{bad}");
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
        assert!(ciq_compiler::compile(&format!("function f(){{{body}}}")).is_err());
    }
    let source = "function f(){var i=0; for(i=0;i<2;++i){do{continue;}while(false);} return i;}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}
