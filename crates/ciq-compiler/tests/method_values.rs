#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{
    api::{self, ApiContext, ApiLevel, Type},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_function_with_api, compile_sources, parse_function_ir,
    parse_function_ir_with_api, parse_unit_ir,
};

#[test]
fn methods_preserve_erased_contracts_api_and_saved_stages() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>, s as Toybox.Lang.Symbol) as Toybox.Lang.Method { return a.method(s); }",
        "function f(d as Toybox.Lang.Dictionary) as Toybox.Lang.Method { return d.method(:size); }",
        "function f(s as Toybox.Lang.String) as Toybox.Lang.Method { return s.method(:length); }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Method { return o.method(:toString); }",
        "function f(m as Toybox.Lang.Method) as Toybox.Lang.Object or Null { return m.invoke(); }",
        "function f(m as Toybox.Lang.Method) as Toybox.Lang.Object or Null { return m.invoke(1,\"a\",null,[1],{}); }",
        "function f(m as Toybox.Lang.Method) { m.invoke(); return m.hashCode(); }",
        "function f(m as Toybox.Lang.Method) as Toybox.Lang.String { return m.toString(); }",
        "function f(m as Toybox.Lang.Method) as Toybox.Lang.Boolean { return m.equals(m); }",
        "function f(a as Toybox.Lang.Method,b as Toybox.Lang.Method,c as Toybox.Lang.Boolean) as Toybox.Lang.Method { return c ? a : b; }",
        "function f(m as Toybox.Lang.Method) as Toybox.Lang.String { var v=m.invoke(); if(v==null) { return \"null\"; } return v.toString(); }",
    ] {
        let f = compile_function_with_api(source, initial).unwrap();
        assert_eq!(
            f,
            parse_function_ir_with_api(&f.to_ir_text(), initial).unwrap()
        );
        let asm = f.lower();
        assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    }
    let weak =
        "function f(m as Toybox.Lang.Method) as Toybox.Lang.WeakReference { return m.weak(); }";
    assert!(compile_function_with_api(weak, initial).is_err());
    let api = ApiContext::new(ApiLevel(1, 2, 0), ApiLevel(1, 2, 0)).unwrap();
    let f = compile_function_with_api(weak, api).unwrap();
    assert_eq!(f, parse_function_ir_with_api(&f.to_ir_text(), api).unwrap());
}

#[test]
fn variadic_declarations_and_invoke_byte_counts_do_not_weaken_fixed_arity() {
    let method = api::core()
        .method_for_type(&Type::named("Toybox.Lang.Method"), "invoke")
        .unwrap();
    assert!(method.parameters.is_empty());
    assert_eq!(
        method.variadic,
        Some(Type::Union(vec![
            Type::named("Toybox.Lang.Object"),
            Type::Null
        ]))
    );
    assert_eq!(
        method.result,
        Type::Union(vec![Type::named("Toybox.Lang.Object"), Type::Null])
    );
    for n in [0, 1, 2, 254] {
        let arguments = vec!["1"; n].join(",");
        let f=compile_function(&format!("function f(m as Toybox.Lang.Method) as Toybox.Lang.Object or Null {{ return m.invoke({arguments}); }}")).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        assert!(
            f.lower()
                .to_text()
                .contains(&format!("invokem 0x{:02x}", n + 1))
                || n == 0 && f.lower().to_text().contains("invokemz")
        );
    }
    let args = vec!["1"; 255].join(",");
    assert!(
        checking::compile_function(&format!(
            "function f(m as Toybox.Lang.Method) {{ m.invoke({args}); return 0; }}"
        ))
        .is_err()
    );
    for source in [
        "function f() { return 1.abs(1); }",
        "function f() as Toybox.Lang.Method { return [1].method(); }",
        "function f() as Toybox.Lang.Method { return [1].method(1); }",
        "function f() as Toybox.Lang.Method { return [1].method(:size,:size); }",
        "function f(m as Toybox.Lang.Method) { return m.hashCode(1); }",
        "function f(m as Toybox.Lang.Method) { return m.invoke()+1; }",
        "function f(m as Toybox.Lang.Method) as Toybox.Lang.String { return m.invoke().toString(); }",
        "function f(m as Toybox.Lang.Method) as Toybox.Lang.Array<Toybox.Lang.Number> { var v=m.invoke(); if(v==null) { return []; } return v; }",
    ] {
        assert!(
            checking::compile_function(source).is_err(),
            "accepted {source}"
        );
    }
    let text = compile_function(
        "function f(m as Toybox.Lang.Method) as Toybox.Lang.Object or Null { return m.invoke(); }",
    )
    .unwrap()
    .to_ir_text();
    assert!(parse_function_ir(&text.replace("result=nullable-object", "result=boolean")).is_err());
    assert!(parse_function_ir(&text.replace(" invoke", " absent")).is_err());
}

#[test]
fn raw_code_binding_resolves_new_native_members_and_weak_observers() {
    let symbols=ciq_assembly::Program::parse(".symbol method \"method\" id=0x008000b2\n.symbol invoke \"invoke\" id=0x008000ac\n.symbol size \"size\" id=0x008000ae\n.symbol stillAlive \"stillAlive\" id=0x00800417").unwrap();
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Method { return a.method(:size); }",
        "function f(m as Toybox.Lang.Method) as Toybox.Lang.Object or Null { return m.invoke(); }",
        "function f(w as Toybox.Lang.WeakReference) as Toybox.Lang.Boolean { return w.stillAlive(); }",
    ] {
        let asm = compile_function(source).unwrap().lower();
        let bindings = ciq_compiler::symbol_bindings(&asm, &symbols).unwrap();
        assert!(
            asm.encode_with_literals(&bindings, &Default::default(), &Default::default())
                .is_ok()
        );
    }
}

#[test]
fn method_lifetimes_and_results_survive_complete_application_and_text_paths() {
    let unit =
        compile_sources(&[include_str!("../../../examples/compiler-methods/app.mc")]).unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [0x94; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
    let asm = unit.lower();
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
}
