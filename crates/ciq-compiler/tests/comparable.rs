use ciq_compiler::{
    api::{ApiContext, ApiLevel, Type},
    compile, compile_sources, compile_sources_with_api, parse_ir, parse_unit_ir,
    parse_unit_ir_with_api,
};

const APP: &str = include_str!("../../../examples/compiler-comparators/app.mc");
const ORDERING: &str = include_str!("../../../examples/compiler-comparators/ordering.mc");
const COMPARABLE: &str = "Toybox.Lang.Comparable";

fn context(target: ApiLevel, minimum: ApiLevel) -> ApiContext {
    ApiContext::new(target, minimum).unwrap()
}

#[test]
fn scalar_comparable_witnesses_share_api_and_saved_pipeline_checks() {
    let current = context(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0));
    let consumer = "module M { function use(c as Toybox.Lang.Comparable, other as Toybox.Lang.Object) as Toybox.Lang.Number { return c.compareTo(other); } }";
    for (ty, literal) in [
        ("Number", "1"),
        ("Long", "1l"),
        ("Float", "1.0"),
        ("Double", "1.0d"),
        ("Boolean", "true"),
        ("Char", "'a'"),
        ("String", "\"a\""),
    ] {
        let source = format!(
            "function create(value as Toybox.Lang.{ty}) as Toybox.Lang.Comparable {{ return value; }} function use() as Toybox.Lang.Number {{ return M.use({literal},{literal}); }}"
        );
        let unit = compile_sources_with_api(&[consumer, &source], current).unwrap();
        let text = unit.to_ir_text();
        let resumed = parse_unit_ir_with_api(&text, current).unwrap();
        assert_eq!(unit, resumed, "{ty}");
        assert_eq!(unit.lower(), resumed.lower(), "{ty}");
        for low in [
            context(ApiLevel(4, 2, 0), ApiLevel(4, 2, 0)),
            context(ApiLevel(5, 2, 0), ApiLevel(4, 2, 0)),
        ] {
            assert!(
                compile_sources_with_api(&[consumer, &source], low).is_err(),
                "{ty}"
            );
            assert!(parse_unit_ir_with_api(&text, low).is_err(), "{ty}");
        }
        let changed = source.replace(
            "as Toybox.Lang.Comparable { return value; }",
            "as Toybox.Lang.Comparator { return value; }",
        );
        assert_ne!(changed, source);
        assert!(compile_sources(&[consumer, &changed]).is_err(), "{ty}");
        let nullable = format!(
            "function f(value as Toybox.Lang.{ty} or Null) as Toybox.Lang.Comparable {{ return value; }}"
        );
        assert!(compile(&nullable).is_err(), "{ty}");
        let function = compile(&format!(
            "function f() as Toybox.Lang.Comparable {{ return {literal}; }}"
        ))
        .unwrap();
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap(), "{ty}");
    }
}

#[test]
fn scalar_witnesses_do_not_enable_erased_nullable_or_heterogeneous_contracts() {
    for source in [
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Comparable { return o; }",
        "function f(o as Toybox.Lang.Symbol) as Toybox.Lang.Comparable { return o; }",
        "function f(o as Toybox.Lang.Method) as Toybox.Lang.Comparable { return o; }",
        "function f(o as Toybox.Lang.Dictionary) as Toybox.Lang.Comparable { return o; }",
        "function f(o as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Comparable { return o; }",
        "function f(c as Toybox.Lang.Comparable) as Void { var local=c; local=null; local.compareTo(1); }",
        "function f(c as Toybox.Lang.Comparator) as Void { var local=c; local=1; local.compare(1,2); }",
        "function f() as Toybox.Lang.Number { var local=1; local=\"x\"; return local-1; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let function = compile("function f(value as Toybox.Lang.Object) as Toybox.Lang.Comparable { if (value instanceof Toybox.Lang.Number) { return value; } else { return 0; } }").unwrap();
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
    let function = compile(
        "function f(value as Toybox.Lang.Number) as Toybox.Lang.Comparable { return value; }",
    )
    .unwrap();
    let text = function.to_ir_text();
    // Integer-only parameter lists are omitted in the canonical text.
    let changed = text.replace("parameters=1", "parameters=1 parameter-types=object");
    assert_ne!(changed, text);
    assert!(parse_ir(&changed).is_err());
}

#[test]
fn interface_calls_forwarding_locals_and_returns_share_saved_contracts() {
    let api = context(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0));
    let unit = compile_sources_with_api(&[APP, ORDERING], api).unwrap();
    let text = unit.to_ir_text();
    let resumed = parse_unit_ir_with_api(&text, api).unwrap();
    assert_eq!(unit, resumed);
    assert_eq!(unit.lower(), resumed.lower());
    let assembly = unit.lower().to_text();
    assert!(assembly.contains("__ciq_native_compareTo"));
    assert!(assembly.contains("__ciq_native_compare"));
    let low = context(ApiLevel(5, 2, 0), ApiLevel(4, 2, 0));
    assert!(parse_unit_ir_with_api(&text, low).is_err());
    let changed = text.replace(
        ".function create result=comparable",
        ".function create result=comparator",
    );
    assert_ne!(changed, text);
    assert!(parse_unit_ir(&changed).is_err());
    let changed = text.replace(
        ".function compareTo parameters=1 receiver=instance parameter-types=object",
        ".function compareTo parameters=1 receiver=instance parameter-types=integer",
    );
    assert_ne!(changed, text);
    assert!(parse_unit_ir(&changed).is_err());
    let function = compile("import Toybox.Lang; function f(a as Comparable, b as Comparable, choose as Boolean) as Comparable { var c=choose ? a : b; c=a; return c; }").unwrap();
    assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
}

#[test]
fn interface_boundaries_reject_erasure_and_mismatched_members() {
    let consumer = "module M { function use(c as Toybox.Lang.Comparable) as Toybox.Lang.Number { return c.compareTo(1); } function make(c as C) as Toybox.Lang.Comparable { return c; } }";
    for method in [
        "function other(o as Toybox.Lang.Object) as Toybox.Lang.Number { return 0; }",
        "function compareTo(o as Toybox.Lang.Number) as Toybox.Lang.Number { return 0; }",
        "function compareTo(o as Toybox.Lang.Object or Null) as Toybox.Lang.Number { return 0; }",
        "function compareTo(o as C) as Toybox.Lang.Number { return 0; }",
        "function compareTo(o as Toybox.Lang.Object) as Toybox.Lang.Long { return 0l; }",
    ] {
        let class = format!("class C {{ function initialize() {{}} {method} }}");
        assert!(compile_sources(&[consumer, &class]).is_err(), "{class}");
    }
    let consumer = "module M { function use(c as Toybox.Lang.Comparable) as Toybox.Lang.Number { return c.compareTo(1); } }";
    for source in [
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { return M.use(o); }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { if (o has :compareTo) { return M.use(o); } else { return 0; } }",
        "function f() as Toybox.Lang.Number { return M.use(null); }",
    ] {
        assert!(compile_sources(&[consumer, source]).is_err(), "{source}");
    }
    for source in [
        "function f(c as Toybox.Lang.Comparable) as Toybox.Lang.Number { return c.compareTo(null); }",
        "function f(c as Toybox.Lang.Comparable) as Toybox.Lang.Number { return c.compareTo(); }",
        "function f(c as Toybox.Lang.Comparable) as Toybox.Lang.Number { return c.compareTo(1,2); }",
        "function f(c as Toybox.Lang.Comparable) as Toybox.Lang.Number { return c.compare(1,2); }",
        "function f(c as Toybox.Lang.Comparator) as Toybox.Lang.Number { return c.compareTo(1); }",
        "function f(c as Toybox.Lang.Comparable) as Void { var a=[1]; a.sort(c); }",
        "function f(a as Toybox.Lang.Comparable,b as Toybox.Lang.Comparator) as Toybox.Lang.Number { var c=a; c=b; return c.compareTo(1); }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn erased_comparison_effects_preserve_reference_initialization_requirements() {
    let target = "class Target { function initialize() {} }";
    for (interface, call, helper) in [
        ("Comparable", "var n=c.compareTo(1);", ""),
        (
            "Comparable",
            "var n=M.call(c);",
            "module M { function call(c as Toybox.Lang.Comparable) as Toybox.Lang.Number { return c.compareTo(1); } }",
        ),
        ("Comparator", "var a=[1,2]; a.sort(c);", ""),
        ("Comparator or Null", "var a=[1,2]; a.sort(c);", ""),
    ] {
        let before = format!(
            "class Holder {{ var target as Target; function initialize(c as Toybox.Lang.{interface},t as Target) {{ {call} self.target=t; }} }}"
        );
        let error = compile_sources(&[target, helper, &before])
            .unwrap_err()
            .to_string();
        assert!(error.contains("shutdown"), "{error}");
        let after = before.replace(
            &format!("{call} self.target=t;"),
            &format!("self.target=t; {call}"),
        );
        assert!(
            compile_sources(&[target, helper, &after]).is_ok(),
            "{after}"
        );
    }
}

#[test]
fn comparable_is_a_versioned_interface_without_a_runtime_class() {
    let catalogue = ciq_compiler::api::core();
    let declaration = catalogue.interface(COMPARABLE).unwrap();
    assert!(catalogue.class(COMPARABLE).is_none());
    assert_eq!(declaration.since, ApiLevel(5, 0, 0));
    let method = &declaration.methods[0];
    assert_eq!(method.name, "compareTo");
    assert_eq!(method.parameters, [Type::named("Toybox.Lang.Object")]);
    assert_eq!(method.result, Type::named("Toybox.Lang.Number"));
    assert_eq!(method.since, ApiLevel(5, 0, 0));
}

#[test]
fn source_signature_proofs_survive_saved_ir_and_require_api_levels() {
    let unit = compile_sources(&[APP, ORDERING]).unwrap();
    let current = context(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0));
    for candidate in [&unit, &parse_unit_ir(&unit.to_ir_text()).unwrap()] {
        candidate
            .check_interface("TextOrder", COMPARABLE, current)
            .unwrap();
        candidate
            .check_interface("Ordering", "Toybox.Lang.Comparator", current)
            .unwrap();
        assert!(
            candidate
                .check_interface("Ordering", COMPARABLE, current)
                .is_err()
        );
        assert!(
            candidate
                .check_interface("TextOrder", "Toybox.Lang.Comparator", current)
                .is_err()
        );
        for low in [
            context(ApiLevel(4, 2, 0), ApiLevel(4, 2, 0)),
            context(ApiLevel(5, 2, 0), ApiLevel(4, 2, 0)),
        ] {
            let error = candidate
                .check_interface("TextOrder", COMPARABLE, low)
                .unwrap_err();
            assert!(error.contains("requires API 5.0.0"), "{error}");
        }
        for (class, interface) in [
            ("Missing", COMPARABLE),
            ("Toybox.Lang.Number", COMPARABLE),
            ("TextOrder", "Toybox.Lang.Object"),
            ("TextOrder", "Missing"),
        ] {
            assert!(
                candidate
                    .check_interface(class, interface, current)
                    .is_err()
            );
        }
    }
}

#[test]
fn source_comparable_requires_the_exact_member_signature() {
    let current = context(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0));
    for (parameters, result, body, accepted) in [
        (
            "o as Toybox.Lang.Object",
            "Toybox.Lang.Number",
            "return 0;",
            true,
        ),
        (
            "o as Toybox.Lang.Object or Null",
            "Toybox.Lang.Number",
            "return 0;",
            false,
        ),
        (
            "o as Toybox.Lang.Number",
            "Toybox.Lang.Number",
            "return 0;",
            false,
        ),
        ("o as C", "Toybox.Lang.Number", "return 0;", false),
        ("", "Toybox.Lang.Number", "return 0;", false),
        (
            "a as Toybox.Lang.Object,b as Toybox.Lang.Object",
            "Toybox.Lang.Number",
            "return 0;",
            false,
        ),
        (
            "o as Toybox.Lang.Object",
            "Toybox.Lang.Number or Null",
            "return null;",
            false,
        ),
        (
            "o as Toybox.Lang.Object",
            "Toybox.Lang.Long",
            "return 0l;",
            false,
        ),
        ("o as Toybox.Lang.Object", "Void", "return;", false),
    ] {
        let source = format!(
            "class C {{ function initialize() {{}} function compareTo({parameters}) as {result} {{ {body} }} }}"
        );
        let unit = compile_sources(&[&source]).unwrap();
        let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
        for candidate in [&unit, &resumed] {
            assert_eq!(
                candidate.check_interface("C", COMPARABLE, current).is_ok(),
                accepted,
                "{source}"
            );
        }
    }
    let source = "class C { function initialize() {} function compareTo(o as Toybox.Lang.Object) as Toybox.Lang.Number { return 0; } }";
    let unit = compile_sources(&[source]).unwrap();
    let changed = unit
        .to_ir_text()
        .replace("parameter-types=object", "parameter-types=integer");
    assert_ne!(changed, unit.to_ir_text());
    let resumed = parse_unit_ir(&changed).unwrap();
    assert!(resumed.check_interface("C", COMPARABLE, current).is_err());
}

#[test]
fn interface_contracts_do_not_enable_runtime_tests_or_unchecked_casts() {
    for source in [
        "import Toybox.Lang; class Comparable { function initialize() {} }",
        "import Toybox.Lang; function f(o as Object) as Toybox.Lang.Number { return o instanceof Comparable; }",
        "import Toybox.Lang; function f(o as Object) as Toybox.Lang.Number { return o as Comparable; }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}

#[test]
fn native_interface_refinement_covers_each_scalar_and_saved_pipeline() {
    let current = context(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0));
    let low = context(ApiLevel(5, 2, 0), ApiLevel(4, 2, 0));
    for (ty, body) in [
        ("Number", "return c+1;"),
        ("Long", "return (c+1l).toNumber();"),
        ("Float", "return (c+1.0).toNumber();"),
        ("Double", "return (c+1.0d).toNumber();"),
        ("Boolean", "if(!c){return 1;} return 0;"),
        ("Char", "return c.toNumber();"),
        ("String", "return c.length();"),
    ] {
        for nullable in ["", " or Null"] {
            for local in [false, true] {
                let parameter = if local { "input" } else { "c" };
                let prefix = if local { "var c=input;" } else { "" };
                let source = format!(
                    "function f({parameter} as Toybox.Lang.Comparable{nullable}) as Toybox.Lang.Number {{ {prefix} if(c instanceof Toybox.Lang.{ty}){{ {body} }} return 0; }}"
                );
                let function = compile(&source).unwrap_or_else(|e| panic!("{source}: {e}"));
                assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
                function.validate_api(current).unwrap();
                assert!(function.validate_api(low).is_err());
                let asm = function.lower();
                assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
            }
        }
    }
    for body in [
        "if(c instanceof Toybox.Lang.Number){return M.use(c)+c;} return 0;",
        "if(!(c instanceof Toybox.Lang.Number)){return 0;} return M.use(c)+c;",
        "return c instanceof Toybox.Lang.Number ? M.use(c)+c : 0;",
        "var v=c; while(v instanceof Toybox.Lang.Number){var n=M.use(v)+v; v=\"done\"; return n;} return 0;",
        "if(c instanceof Toybox.Lang.Number){var old=c; c=\"done\"; return old+1;} return 0;",
    ] {
        let source = format!(
            "module M {{function use(c as Toybox.Lang.Comparable) as Toybox.Lang.Number {{return c.compareTo(1);}}}} function f(c as Toybox.Lang.Comparable or Null) as Toybox.Lang.Number {{{body}}}"
        );
        let unit = compile_sources_with_api(&[&source], current).unwrap();
        assert_eq!(
            unit,
            parse_unit_ir_with_api(&unit.to_ir_text(), current).unwrap()
        );
    }
}

#[test]
fn interface_refinement_requires_live_compatible_class_facts() {
    for source in [
        "function f(c as Toybox.Lang.Comparable) as Toybox.Lang.Number {return c+1;}",
        "function f(c as Toybox.Lang.Comparable) as Toybox.Lang.Number {if(c instanceof Toybox.Lang.Number){} return c+1;}",
        "function f(c as Toybox.Lang.Comparable) as Toybox.Lang.Number {if(c instanceof Toybox.Lang.Number){c=\"done\"; return c+1;} return 0;}",
        "function f(c as Toybox.Lang.Comparable) as Toybox.Lang.Number {var v=c; if(v instanceof Toybox.Lang.Number){v=\"done\"; return v+1;} return 0;}",
        "function f(c as Toybox.Lang.Comparable) as Toybox.Lang.Number {var copy=c; if(c instanceof Toybox.Lang.Number){return copy+1;} return 0;}",
        "function f(c as Toybox.Lang.Comparator) as Toybox.Lang.Number {if(c instanceof Toybox.Lang.Number){return c+1;} return 0;}",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    let function=compile("function f(c as Toybox.Lang.Comparable) as Toybox.Lang.Number {if(c instanceof Toybox.Lang.Number){return c+1;} return 0;}").unwrap();
    let text = function.to_ir_text();
    let forged = text.replace("Toybox.Lang.Number", "Toybox.Lang.String");
    assert_ne!(text, forged);
    assert!(parse_ir(&forged).is_err());
}
