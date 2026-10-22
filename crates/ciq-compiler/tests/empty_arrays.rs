use ciq_compiler::{
    ArrayElement, Operation, ReturnType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_function_with_api, compile_sources, parse_function_ir,
    parse_function_ir_with_api, parse_unit_ir,
};

#[test]
fn contextual_empty_arrays_preserve_all_pipeline_boundaries() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-empty-arrays/app.mc"
    )])
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [46; 16],
        entry: "start".into(),
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
    for (name, element) in [
        ("Number", ArrayElement::Number),
        ("Char", ArrayElement::Char),
    ] {
        let f = compile_function(&format!(
            "function empty() as Toybox.Lang.Array<Toybox.Lang.{name}> {{ return []; }}"
        ))
        .unwrap();
        assert_eq!(f.return_type(), ReturnType::Array(element.clone()));
        assert_eq!(f.operations().len(), 2);
        assert!(
            matches!(f.operations()[0].operation, Operation::ArrayNew { ref element_type, .. } if *element_type == element)
        );
        assert_eq!(f.lower().encode().unwrap(), [0x35, 1, 0x37, 0x14, 0x16]);
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    let sources = [
        "function start() { return M.count([]); }",
        "module M { function count(a as Toybox.Lang.Array<Toybox.Lang.Number>) { return a.size(); } }",
    ];
    let unit = compile_sources(&sources).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let source = "class App extends Application.AppBase { function count(a as Toybox.Lang.Array<Toybox.Lang.Char>) { return a.size(); } function run() { self.count([]); return self.count([]); } }";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let source = "module M { function count(a as Toybox.Lang.Array<Toybox.Lang.Number>, b as Toybox.Lang.Boolean) { if (b) { return a.size(); } return 0; } } function start() { return M.count([], true || false); }";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a.reverse().addAll([]); return a.size(); }",
        "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.Array<Toybox.Lang.Number> { if (b) { return []; } return [1]; }",
        "function f(b as Toybox.Lang.Boolean) { var a = [1]; while (b) { a = []; break; } return a.size(); }",
    ] {
        let f = compile_function(source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
}

#[test]
fn expected_contracts_do_not_widen_elements_or_hide_bad_calls() {
    for source in [
        "function f() { var a = []; }",
        "function f() { return []; }",
        "function f() as Toybox.Lang.Number { return []; }",
        "function f() { var n = 1; n = []; }",
        "function f() { var a = [1]; a.add([]); }",
        "function f() { var a = [1]; a.unknown([]); }",
        "function f() { var a = [1]; a.addAll([], 1); }",
        "function f() { var a = [1]; a.addAll(['a']); }",
        "function f() { var a = ['a']; a = [1]; a.add('b'); }",
        "function f() { return [].size(); }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number> { return [].reverse(); }",
        "function f() { var a = [[], []]; }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    for source in [
        "module M { function count(n) { return n; } } function start() { return M.count([]); }",
        "module M { function count(a as Toybox.Lang.Array<Toybox.Lang.Char>) { return a.size(); } } function start() { return M.count([1]); }",
        "module M { function count(a as Toybox.Lang.Array<Toybox.Lang.Number>) { return a.size(); } } function start() { return M.count([], 1); }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let source =
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) { a.addAll([]); return a.size(); }";
    let f = compile_function(source).unwrap();
    let text = f.to_ir_text();
    assert!(text.contains(".array-new %"));
    let changed = text
        .lines()
        .map(|line| {
            if line.starts_with(".array-new ") {
                line.replacen("number", "char", 1)
            } else {
                line.into()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(changed, text.trim_end());
    assert!(parse_function_ir(&changed).is_err());
}

#[test]
fn array_construction_does_not_inherit_append_api_requirements() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let source = "function f() as Toybox.Lang.Array<Toybox.Lang.Number> { return []; }";
    let f = compile_function_with_api(source, initial).unwrap();
    assert_eq!(
        f,
        parse_function_ir_with_api(&f.to_ir_text(), initial).unwrap()
    );
    assert!(
        compile_function_with_api(
            "function f() as Toybox.Lang.Array<Toybox.Lang.Number> { return [1]; }",
            initial
        )
        .is_ok()
    );
    assert!(
        compile_function_with_api(
            "function f() as Toybox.Lang.Array<Toybox.Lang.Char> { return []; }",
            initial
        )
        .is_err()
    );
}
