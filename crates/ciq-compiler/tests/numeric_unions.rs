use ciq_compiler::{
    ParameterType, ReturnType,
    api::{ApiContext, ApiLevel, Type, core},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources, compile_sources_with_api, parse_function_ir, parse_unit_ir,
};

#[test]
fn numeric_example_preserves_saved_stages_and_complete_application() {
    let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[include_str!(
            "../../../examples/compiler-numeric-unions/app.mc"
        )],
        context,
    )
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [92; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
}

#[test]
fn alias_signatures_casts_calls_and_common_methods_round_trip() {
    for (name, tag, parameter, result, api_type) in [
        (
            "Integer",
            "integer-union",
            ParameterType::IntegerAlias,
            ReturnType::IntegerAlias,
            Type::integer(),
        ),
        (
            "Decimal",
            "decimal",
            ParameterType::Decimal,
            ReturnType::Decimal,
            Type::decimal(),
        ),
        (
            "Numeric",
            "numeric",
            ParameterType::Numeric,
            ReturnType::Numeric,
            Type::numeric(),
        ),
    ] {
        assert_eq!(
            core().type_alias(&format!("Toybox.Lang.{name}")),
            Some(api_type.clone())
        );
        assert_eq!(
            api_type.clone().with_null().with_null(),
            api_type.with_null()
        );
        let source = format!(
            "using Toybox.Lang as L; function f(n as L.{name}) as L.{name} {{ var local=n; local=1{}; return +(-local); }}",
            if name == "Decimal" { ".5" } else { "" }
        );
        let function = compile_function(&source).unwrap();
        assert_eq!(function.parameter_types(), &[parameter]);
        assert_eq!(function.return_type(), result);
        assert!(function.to_ir_text().contains(&format!("result={tag}")));
        assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
        for nullable in ["", " or Null"] {
            let source = format!(
                "function f(o as Lang.Object) as Lang.{name}{nullable} {{ return o as Lang.{name}{nullable}; }}"
            );
            let f = compile_function(&source).unwrap();
            assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
            assert!(!f.lower().to_text().contains("isa"));
        }
        let source = format!(
            "function f(n as Lang.{name}) as Void {{ System.println(n);n.toString();n.toNumber();n.toLong();n.toFloat();n.toDouble();n.format(\"%f\"); }}"
        );
        let f = compile_function(&source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        let source = format!(
            "module M {{function f(n as Lang.{name}) as Lang.{name} or Null {{return n;}} function g(n as Lang.{name}) as Lang.{name} or Null {{return M.f(n);}}}}"
        );
        let unit = compile_sources(&[&source]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
}

#[test]
fn alias_operators_use_vm_operations_without_conversions() {
    for (name, expr, result, opcode) in [
        ("Numeric", "a+b", "Numeric", "addv"),
        ("Numeric", "a-b", "Numeric", "subv"),
        ("Numeric", "a*b", "Numeric", "mulv"),
        ("Numeric", "a/b", "Numeric", "divv"),
        ("Integer", "a%b", "Integer", "modv"),
        ("Integer", "a&b", "Integer", "andv"),
        ("Integer", "a|b", "Integer", "orv"),
        ("Integer", "a^b", "Integer", "xorv"),
        ("Integer", "a<<b", "Integer", "shlv"),
        ("Integer", "a>>b", "Integer", "shrv"),
        ("Decimal", "a+b", "Decimal", "addv"),
    ] {
        let source = format!(
            "function f(a as Lang.{name},b as Lang.{name}) as Lang.{result} {{return {expr};}}"
        );
        let f = compile_function(&source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        let text = f.lower().to_text();
        assert!(text.contains(opcode), "{text}");
        assert!(
            !text.contains("getmv") && !text.contains("invokem") && !text.contains("isa"),
            "{text}"
        );
    }
    for source in [
        "function f(a as Lang.Numeric) as Lang.Double {return a+1.0d;}",
        "function f(a as Lang.Numeric) as Lang.Decimal {return a+1.0;}",
        "function f(a as Lang.Decimal) as Lang.Double {return a+1l;}",
        "function f(a as Lang.Integer) as Lang.Decimal {return a+1.0;}",
        "function f(a as Lang.Integer) as Lang.Char {return 'a'+a;}",
        "function f(a as Lang.Integer) as Lang.Integer {return ~a;}",
        "function f(a as Lang.Decimal) as Lang.Decimal {var x=a; ++x; x--; return x;}",
        "function f(a as Lang.Numeric,b as Lang.Numeric) as Lang.Boolean {return a<=b;}",
        "function f(a as Lang.Integer) {if(a instanceof Lang.Number){return a;} return 0;}",
    ] {
        let f = compile_function(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    let source = "function f(a as Lang.Numeric,b as Lang.Numeric) as Lang.Numeric {return a+b;}";
    let f = compile_function(source).unwrap();
    assert!(
        parse_function_ir(
            &f.to_ir_text()
                .replace("result=numeric", "result=integer-union")
        )
        .is_err()
    );
}

#[test]
fn nullable_aliases_require_current_proofs_and_preserve_join_contracts() {
    for name in ["Numeric", "Integer", "Decimal"] {
        for body in [
            "if(n==null){return null;} return n+1;",
            "return n!=null ? n+1 : null;",
            "var x=n; if(x!=null){x=x+1;} return x;",
        ] {
            let source =
                format!("function f(n as Lang.{name} or Null) as Lang.{name} or Null {{{body}}}");
            let f = compile_function(&source).unwrap();
            assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
            let text = f.to_ir_text();
            if text.contains(".not-equal ") {
                assert!(parse_function_ir(&text.replace(".not-equal ", ".equal ")).is_err());
            }
        }
        let alternate = if name == "Decimal" { "1.5" } else { "1" };
        let source = format!(
            "function f(n as Lang.{name},flag as Lang.Boolean) as Lang.{name} {{return flag?n:{alternate};}}"
        );
        let f = compile_function(&source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        for body in [
            "return n+1;",
            "if(n!=null){var x=n; x=null; return x+1;} return null;",
            "var ok=n!=null; if(ok){return n+1;} return null;",
        ] {
            let source =
                format!("function f(n as Lang.{name} or Null) as Lang.{name} or Null {{{body}}}");
            assert!(compile_function(&source).is_err(), "{source}");
        }
    }
}

#[test]
fn alias_domains_do_not_admit_null_or_unsupported_concrete_pairs() {
    for source in [
        "function f(a as Lang.Numeric,b as Lang.Numeric) as Lang.Numeric {return a%b;}",
        "function f(a as Lang.Decimal,b as Lang.Integer) as Lang.Numeric {return a&b;}",
        "function f(a as Lang.Numeric) as Lang.Numeric {return ~a;}",
        "function f(a as Lang.Numeric) as Lang.Char {return 'a'+a;}",
        "function f(a as Lang.Integer) as Lang.Boolean {return 'a'<a;}",
        "function f(a as Lang.Integer) as Lang.Integer {var x=a; x=1.5; return x;}",
        "function f(a as Lang.Decimal) as Lang.Decimal {var x=a; x=1; return x;}",
        "function f(a as Lang.Numeric) as Lang.Numeric {var x=a; x=null; return x;}",
        "function f(a as Lang.Decimal) as Lang.Number {return a;}",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
}
