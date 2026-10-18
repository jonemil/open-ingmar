use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, compile_with_api, parse_ir, parse_ir_with_api, parse_unit_ir,
};

#[test]
fn symbols_bind_by_name_without_becoming_numbers_or_strings() {
    let f = compile("function f() as Toybox.Lang.Symbol { return :alpha; }").unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let asm = f.lower();
    assert!(asm.to_text().contains("spush @__ciq_symbol_alpha"));
    let symbols = ciq_assembly::Program::parse(".symbol alpha \"alpha\" id=0x01020304").unwrap();
    let bindings = ciq_compiler::symbol_bindings(&asm, &symbols).unwrap();
    let code = asm
        .encode_with_literals(&bindings, &Default::default(), &Default::default())
        .unwrap();
    assert!(code.windows(5).any(|w| w == [0x27, 1, 2, 3, 4]));
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    assert!(
        ciq_compiler::symbol_bindings(
            &asm,
            &ciq_assembly::Program::parse(".symbol other \"other\" id=0x1").unwrap()
        )
        .is_err()
    );
    assert!(
        ciq_compiler::symbol_bindings(
            &asm,
            &ciq_assembly::Program::parse(".symbol a \"alpha\" id=0x1\n.symbol b \"alpha\" id=0x2")
                .unwrap()
        )
        .is_err()
    );
}

#[test]
fn symbol_contracts_keep_api_boundaries_user_calls_joins_and_native_methods() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    for source in [
        "function f(s as Toybox.Lang.Symbol) as Toybox.Lang.Symbol { return s; }",
        "function f(s as Toybox.Lang.Symbol) as Toybox.Lang.String { return s.toString(); }",
        "function f(s as Toybox.Lang.Symbol) as Toybox.Lang.Boolean { return s.equals(:alpha); }",
        "function f(s as Toybox.Lang.Symbol) { return s.hashCode(); }",
        "function f(s as Toybox.Lang.Symbol) as Toybox.Lang.Boolean { return s==:alpha; }",
        "function f(s as Toybox.Lang.Symbol) as Toybox.Lang.Boolean { return s!=:alpha; }",
        "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Symbol { return flag ? :alpha : :beta; }",
        "function f() as Toybox.Lang.Symbol { var s=:alpha; s=:beta; return s; }",
        "function f() as Toybox.Lang.Object { return :alpha; }",
    ] {
        let f = compile_with_api(source, initial).unwrap();
        assert_eq!(f, parse_ir_with_api(&f.to_ir_text(), initial).unwrap());
    }
    for (source, since) in [
        (
            "function f(s as Toybox.Lang.Symbol) as Toybox.Lang.Symbol { return s.weak(); }",
            ApiLevel(1, 2, 0),
        ),
        (
            "function f(s as Toybox.Lang.Symbol) { return s.toNumber(); }",
            ApiLevel(2, 3, 0),
        ),
    ] {
        let api = ApiContext::new(since, since).unwrap();
        let f = compile_with_api(source, api).unwrap();
        assert_eq!(f, parse_ir_with_api(&f.to_ir_text(), api).unwrap());
        assert!(compile_with_api(source, initial).is_err());
        assert!(parse_ir_with_api(&f.to_ir_text(), initial).is_err());
    }
    let f = compile("function f(s as Toybox.Lang.Symbol) { return s.weak().toNumber(); }").unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
}

#[test]
fn symbols_reject_numeric_operators_containers_and_invalid_method_binding() {
    for expression in [
        ":alpha+1",
        ":alpha==1",
        ":alpha<:beta",
        "!:alpha",
        "~:alpha",
        ":alpha.toNumber(1)",
        ":alpha.compareTo(:beta)",
        "1.compareTo(:alpha)",
        ":alpha.method(:toNumber, :size)",
    ] {
        assert!(
            compile(&format!("function f() {{ return {expression}; }}")).is_err(),
            "accepted {expression}"
        );
    }
    for source in [
        "function f() { return :; }",
        "function f() { return :123; }",
        "function f() { var s=:alpha; s=1; return 0; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Symbol> { return [:alpha]; }",
        "function f() { return {:alpha=>1}; }",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    for name in ["123", "a.b", "a/b", "\"\"", "unknown-name"] {
        assert!(
            parse_ir(&format!(
                "ciq-ir\n.function f result=symbol\n.symbol %0 {name}\n.return %0\n.end\n"
            ))
            .is_err()
        );
    }
    let text = compile("function f() as Toybox.Lang.Symbol { return :alpha; }")
        .unwrap()
        .to_ir_text();
    assert!(parse_ir(&text.replace("result=symbol", "result=long")).is_err());
}

#[test]
fn symbol_names_and_native_ids_survive_complete_application_and_saved_stages() {
    let unit =
        compile_sources(&[include_str!("../../../examples/compiler-symbols/app.mc")]).unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [0xe5; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    let prg = build_console_application(&unit, &options).unwrap();
    assert_eq!(prg, build_console_application(&resumed, &options).unwrap());
    let section = prg
        .sections()
        .iter()
        .find(|s| s.kind() == ciq_prg::SYMBOLS)
        .unwrap();
    let symbols = ciq_prg::symbols::Symbols::parse(section.payload()).unwrap();
    assert_eq!(symbols.get(0x008000ae).map(|s| s.name), Some("size"));
    let names: Vec<_> = symbols.iter().map(|s| s.name).collect();
    assert!(names.contains(&"alpha") && names.contains(&"beta"));
}
