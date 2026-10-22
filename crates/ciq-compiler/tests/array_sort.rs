use ciq_compiler::{
    ReturnType,
    api::{self, ApiContext, ApiLevel, Type},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_function_with_api, compile_sources_with_api, parse_function_ir,
    parse_function_ir_with_api, parse_unit_ir_with_api,
};

fn context() -> ApiContext {
    ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap()
}

#[test]
fn default_sort_checks_all_scalar_profiles_and_preserves_void_lowering() {
    for ty in [
        "Number", "Long", "Float", "Double", "Boolean", "Char", "String",
    ] {
        let source = format!(
            "function f(a as Toybox.Lang.Array<Toybox.Lang.{ty}>) as Void {{ a.sort(null); }}"
        );
        let f = compile_function_with_api(&source, context()).unwrap();
        assert_eq!(f.return_type(), ReturnType::Void);
        assert_eq!(
            f,
            parse_function_ir_with_api(&f.to_ir_text(), context()).unwrap()
        );
        let asm = f.lower();
        assert!(asm.to_text().contains("@__ciq_native_sort"));
        assert!(asm.to_text().contains("invokem 0x02"));
        let opcodes = asm
            .items()
            .iter()
            .filter_map(|item| match item {
                ciq_assembly::Item::Instruction { opcode, .. } => Some(*opcode),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            &opcodes[opcodes.len() - 3..],
            &[
                ciq_mc::Opcode::Invokem,
                ciq_mc::Opcode::Popv,
                ciq_mc::Opcode::Return
            ]
        );
        assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    }
}

#[test]
fn sort_rejects_unavailable_apis_incompatible_elements_arguments_and_void_consumers() {
    let source = "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a.sort(null); }";
    let f = compile_function_with_api(source, context()).unwrap();
    for api in [
        ApiContext::new(ApiLevel(4, 2, 0), ApiLevel(4, 2, 0)).unwrap(),
        ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(4, 2, 0)).unwrap(),
    ] {
        assert!(
            compile_function_with_api(source, api)
                .unwrap_err()
                .to_string()
                .contains("5.0.0")
        );
        assert!(parse_function_ir_with_api(&f.to_ir_text(), api).is_err());
    }
    for ty in [
        "Number", "Long", "Float", "Double", "Boolean", "Char", "String",
    ] {
        let source = format!(
            "function f(a as Toybox.Lang.Array<Toybox.Lang.{ty} or Null>) {{ a.sort(null); }}"
        );
        assert!(compile_function(&source).is_err(), "accepted {source}");
    }
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.WatchUi.View>) { a.sort(null); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a.sort(); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a.sort(null,null); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a.sort(1); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>,m as Toybox.Lang.Method) { a.sort(m); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { var sorted=a.sort(null); return sorted; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { return a.sort(null); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a.sort(null).size(); }",
    ] {
        assert!(compile_function(source).is_err(), "accepted {source}");
    }
    let text = f.to_ir_text();
    assert!(parse_function_ir(&text.replace("number-array", "nullable-number-array")).is_err());
}

#[test]
fn comparator_declaration_is_an_interface_with_independent_availability() {
    let api = api::core();
    let comparator = api.interface("Toybox.Lang.Comparator").unwrap();
    assert!(api.class(&comparator.name).is_none());
    assert_eq!(
        api.interface_level(&comparator.name),
        Some(ApiLevel(5, 0, 0))
    );
    assert!(context().check_class(api, &comparator.name).is_err());
    assert!(
        context()
            .check_type(api, &Type::named(&comparator.name))
            .is_ok()
    );
    let old = ApiContext::new(ApiLevel(4, 2, 0), ApiLevel(4, 2, 0)).unwrap();
    assert!(old.check_type(api, &Type::named(&comparator.name)).is_err());
    let compare = &comparator.methods[0];
    assert_eq!(compare.name, "compare");
    assert_eq!(
        compare.parameters,
        vec![Type::named("Toybox.Lang.Object"); 2]
    );
    assert_eq!(compare.result, Type::named("Toybox.Lang.Number"));
    let sort = api
        .method_for_type(
            &Type::Array(Box::new(Type::named("Toybox.Lang.Number"))),
            "sort",
        )
        .unwrap();
    assert_eq!(
        sort.parameters,
        vec![Type::Union(vec![Type::named(&comparator.name), Type::Null])]
    );
    assert_eq!(sort.result, Type::Void);
}

#[test]
fn sorting_retains_complete_application_and_saved_stage_parity() {
    let unit = compile_sources_with_api(
        &[include_str!("../../../examples/compiler-array-sort/app.mc")],
        context(),
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [0x96; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
    let asm = unit.lower();
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    let raw = compile_function(
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a.sort(null); }",
    )
    .unwrap()
    .lower();
    let symbols = ciq_assembly::Program::parse(".symbol sort \"sort\" id=0x00800e51\n").unwrap();
    let bindings = ciq_compiler::symbol_bindings(&raw, &symbols).unwrap();
    assert!(
        raw.encode_with_literals(&bindings, &Default::default(), &Default::default())
            .is_ok()
    );
}
