use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    compile, compile_sources, compile_sources_with_api, compile_with_api, parse_ir, parse_unit_ir,
};

#[test]
fn fragment_and_unit_preserve_the_same_parsed_body() {
    let source = "using Toybox.Lang as L;\nfunction f(n as L.Number) as L.Number {\nvar x = n + 1; while (x < 4) { x += 1; } return x;\n}";
    let fragment = compile(source).unwrap();
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(fragment, unit.declarations()[0].function);
    assert_eq!(fragment, parse_ir(&fragment.to_ir_text()).unwrap());
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());

    let context = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    assert_eq!(fragment, compile_with_api(source, context).unwrap());
    assert_eq!(unit, compile_sources_with_api(&[source], context).unwrap());
}

#[test]
fn fragments_restrict_declaration_shape_after_shared_parsing() {
    for source in [
        "",
        "using Toybox.Lang;",
        "function f() {} function g() {}",
        "module M { function f() {} }",
        "class C {}",
        "function f() {} class C {}",
    ] {
        let error = compile(source).unwrap_err();
        assert!(
            error.message.contains("exactly one global function"),
            "{error}"
        );
    }
    for source in [
        "function f() {} function g() {}",
        "module M { function f() {} }",
        "class C {}",
    ] {
        assert!(compile_sources(&[source]).is_ok(), "{source}");
    }
    let invalid = "function f(n as Toybox.Lang.Number) { return n + ; }";
    assert_eq!(
        compile(invalid).unwrap_err(),
        compile_sources(&[invalid]).unwrap_err().diagnostic
    );
}

#[test]
fn external_fragment_calls_do_not_claim_unit_validation() {
    let source = "function f() { return Numbers.left(); }";
    let fragment = compile(source).unwrap();
    assert!(fragment.lower().encode().is_err());
    assert_eq!(fragment, parse_ir(&fragment.to_ir_text()).unwrap());
    assert!(compile_sources(&[source]).is_err());

    let callee = "module Numbers { function left() { return 41; } }";
    let unit = compile_sources(&[source, callee]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let wrapped = format!("ciq-unit\n.owner global\n{}", fragment.to_ir_text());
    assert!(parse_unit_ir(&wrapped).is_err());
}
