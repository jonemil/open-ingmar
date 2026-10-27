use ciq_compiler::{
    ParameterType, ReturnType,
    api::{ApiContext, ApiLevel},
    compile_function, compile_function_with_api, compile_sources, parse_function_ir, parse_unit_ir,
};

#[test]
fn complete_typedef_application_agrees_through_saved_ir() {
    let unit = compile_sources(&[
        include_str!("../../../examples/compiler-typedefs/app.mc"),
        include_str!("../../../examples/compiler-typedefs/types.mc"),
    ])
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    let application = ciq_compiler::application::ConsoleApplication {
        id: [1; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        ciq_compiler::application::build_console_application(&unit, &application)
            .unwrap()
            .to_bytes(),
        ciq_compiler::application::build_console_application(&resumed, &application)
            .unwrap()
            .to_bytes(),
    );
}

#[test]
fn aliases_resolve_across_files_owners_and_declaration_environments() {
    let sources = [
        "function f(n as Types.Item) as Types.Item { return n; }",
        "using Toybox.Lang as L; typedef Shared as Types.Item; module Types { typedef Item as Later; typedef Later as L.Number or L.String; }",
        "class C { typedef Item as Toybox.Lang.Boolean; function own(n as Item) as Item {return n;} function outer(n as Shared) as Shared {return n;} }",
    ];
    let unit = compile_sources(&sources).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert!(matches!(
        unit.declarations()[0].function.return_type(),
        ReturnType::Union(_)
    ));
    assert_eq!(
        unit.classes()[0]
            .methods
            .iter()
            .find(|m| m.name() == "own")
            .unwrap()
            .return_type(),
        ReturnType::Boolean
    );
    assert!(!unit.to_ir_text().contains("typedef"));
    let fragment = compile_function(
        "typedef Value as Toybox.Lang.Number; function f(n as Value) as Value {return n;}",
    )
    .unwrap();
    assert_eq!(fragment.parameter_types(), &[ParameterType::Integer]);
    assert_eq!(fragment, parse_function_ir(&fragment.to_ir_text()).unwrap());
}

#[test]
fn container_field_and_source_union_aliases_retain_existing_contracts() {
    let source = "using Toybox.Lang; typedef Scalar as Lang.Number; typedef Values as Lang.Array<Scalar>; typedef Mapping as Lang.Dictionary<Lang.String, Scalar or Null>; typedef Objects as Token or Lang.String; typedef Maybe as Objects or Null; class Token {} class C {var value as Maybe; function store(v as Maybe) as Void { self.value=v; } function load() as Maybe {return self.value;}} function array(a as Values) as Values {return a;} function map(a as Mapping) as Mapping {return a;}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let field = &unit
        .classes()
        .iter()
        .find(|c| c.name == "C")
        .unwrap()
        .fields[0]
        .kind;
    assert!(matches!(
        field,
        ciq_compiler::FieldKind::Value {
            value_type: ReturnType::SourceUnion(_)
        }
    ));
    let wrong = source.replace("self.value=v", "self.value=41");
    assert!(compile_sources(&[&wrong]).is_err());
    compile_sources(&[
        "typedef T as Token; class Token {} function f(n as Toybox.Lang.Array<T>) {} ",
    ])
    .unwrap();
}

#[test]
fn alias_canonicalization_matches_explicit_types_and_existing_casts() {
    let aliases = "using Toybox.Lang; typedef Item as Lang.Number; typedef Optional as Item or Null; function f(n) as Optional {return n as Optional;}";
    let expanded = "using Toybox.Lang; function f(n) as Lang.Number or Null {return n as Lang.Number or Null;}";
    assert_eq!(
        compile_function(aliases).unwrap().lower(),
        compile_function(expanded).unwrap().lower()
    );
    let exact = "typedef Item as Token; class Token {} function f(n) as Item {return n as Item;}";
    let unit = compile_sources(&[exact]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert!(
        compile_function("typedef T as Toybox.Lang.Number; function f(n) {return n instanceof T;}")
            .is_err()
    );
    assert!(
        compile_function(
            "typedef T as Toybox.Lang.Number or Toybox.Lang.String; function f(n) {return n as T;}"
        )
        .is_ok()
    );
}

#[test]
fn duplicate_unknown_and_cyclic_aliases_fail_even_when_unused() {
    for (source, message) in [
        ("typedef A as A; function f() {}", "recursive typedef"),
        (
            "typedef A as B; typedef B as A; function f() {}",
            "recursive typedef",
        ),
        (
            "typedef A as Missing; function f() {}",
            "unknown typedef target",
        ),
        (
            "typedef A as Toybox.Lang.Number; typedef A as Toybox.Lang.String; function f() {}",
            "duplicate typedef",
        ),
        ("typedef A as Toybox.Lang.Number; class A {}", "conflicts"),
        (
            "typedef A as Void; function f() {}",
            "unsupported parameter",
        ),
    ] {
        let error = compile_sources(&[source]).unwrap_err();
        assert!(error.diagnostic.message.contains(message), "{error}");
    }
    let error = compile_sources(&["function f() {}", "typedef Broken as Unknown;"]).unwrap_err();
    assert_eq!(error.source, 1);
    assert_eq!(error.diagnostic.offset, 8);
    let source = "typedef Same as Toybox.Lang.Number; function Same() {return 1;}";
    assert!(compile_sources(&[source]).is_ok());
}

#[test]
fn expansion_is_bounded_and_shared_dependencies_are_cached() {
    let deep = (0..70)
        .map(|i| format!("typedef A{i} as A{};", i + 1))
        .collect::<String>()
        + "typedef A70 as Toybox.Lang.Number; function f(){}";
    assert!(compile_sources(&[&deep]).is_err());
    let nested = format!(
        "function f(n as {}Toybox.Lang.Number{}) {{}}",
        "Toybox.Lang.Array<".repeat(100),
        ">".repeat(100)
    );
    assert!(
        compile_function(&nested)
            .unwrap_err()
            .message
            .contains("64 levels")
    );
    let diamond = "typedef A0 as Toybox.Lang.Number;".to_owned()
        + &(1..60)
            .map(|i| format!("typedef A{i} as A{} or A{};", i - 1, i - 1))
            .collect::<String>()
        + "function f(n as A59) as A59 {return n;}";
    assert_eq!(
        compile_function(&diamond).unwrap().return_type(),
        ReturnType::Integer
    );
}

#[test]
fn aliases_check_api_availability_even_when_unused() {
    let source = "typedef Bytes as Toybox.Lang.ByteArray; function f() {}";
    let early = ApiContext::new(ApiLevel(2, 4, 0), ApiLevel(2, 4, 0)).unwrap();
    assert!(
        compile_function_with_api(source, early)
            .unwrap_err()
            .message
            .contains("unavailable")
    );
    let supported = ApiContext::new(ApiLevel(3, 0, 0), ApiLevel(3, 0, 0)).unwrap();
    assert!(compile_function_with_api(source, supported).is_ok());
}
