use ciq_compiler::{
    ClassBase, ReturnType, TupleShape,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_sources, parse_unit_ir,
};

const APP: &str = include_str!("../../../examples/compiler-input-counter/app.mc");
const DELEGATE: &str = include_str!("../../../examples/compiler-input-counter/delegate.mc");
const VIEW: &str = include_str!("../../../examples/compiler-input-counter/view.mc");

#[test]
fn input_counter_preserves_callback_contracts_and_complete_pipelines() {
    let unit = compile_sources(&[APP, DELEGATE, VIEW]).unwrap();
    let delegate = &unit.classes()[1];
    assert_eq!(delegate.base, ClassBase::WatchUiBehaviorDelegate);
    let select = delegate
        .methods
        .iter()
        .find(|f| f.name() == "onSelect")
        .unwrap();
    assert_eq!(select.return_type(), ReturnType::Boolean);
    assert_eq!(select.parameter_count(), 0);
    let initial = unit.classes()[0]
        .methods
        .iter()
        .find(|f| f.name() == "getInitialView")
        .unwrap();
    assert_eq!(
        initial.return_type(),
        ReturnType::Tuple(TupleShape::ViewDelegate)
    );
    let text = unit.to_ir_text();
    assert!(text.contains("CounterDelegate WatchUi.BehaviorDelegate"));
    let resumed = parse_unit_ir(&text).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [54; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    let prg = build_console_application(&unit, &options).unwrap();
    assert_eq!(prg, build_console_application(&resumed, &options).unwrap());
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    let imports = prg.unique_section(0xc1a557b1).unwrap().payload();
    assert_eq!(&imports[..2], &3u16.to_be_bytes());
    assert_eq!(
        &imports[18..],
        &[0x00, 0x80, 0x00, 0x11, 0x00, 0x80, 0x00, 0x64]
    );
    ciq_mc::decode(prg.unique_section(ciq_prg::CODE).unwrap().payload()).unwrap();
}

#[test]
fn source_and_saved_ir_reject_invalid_delegates_and_result_slots() {
    for changed in [
        APP.replace(
            "[view, new CounterDelegate(view)]",
            "[new CounterDelegate(view), view]",
        ),
        APP.replace("[view, new CounterDelegate(view)]", "[view, view]"),
        APP.replace("[view, new CounterDelegate(view)]", "[view, null]"),
        APP.replace(
            "[view, new CounterDelegate(view)]",
            "[view, new CounterDelegate(view), view]",
        ),
        APP.replace("new CounterDelegate(view)", "new CounterDelegate(1)"),
        APP.replace("new CounterView()", "new CounterDelegate(1)"),
    ] {
        assert!(
            compile_sources(&[&changed, DELEGATE, VIEW]).is_err(),
            "{changed}"
        );
    }
    for changed in [
        DELEGATE.replace("onSelect()", "onSelect(event)"),
        DELEGATE.replace("return true", "return 1"),
        DELEGATE.replace("BehaviorDelegate.initialize()", "View.initialize()"),
        DELEGATE.replace("self.target = view;", ""),
        DELEGATE.replace("WatchUi.requestUpdate()", "WatchUi.requestUpdate(1)"),
        DELEGATE.replace("System.println(\"select\");", "self.onSelect();"),
        DELEGATE.replace(
            "self.target = view;",
            "self.target = new CounterDelegate(view);",
        ),
    ] {
        assert!(
            compile_sources(&[APP, &changed, VIEW]).is_err(),
            "{changed}"
        );
    }
    let text = compile_sources(&[APP, DELEGATE, VIEW])
        .unwrap()
        .to_ir_text();
    let changed = text
        .lines()
        .map(|line| {
            if line.starts_with(".new-source ") && line.contains("CounterDelegate") {
                line.replace("WatchUi.BehaviorDelegate", "WatchUi.View")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(parse_unit_ir(&changed).is_err());
    let changed = text
        .lines()
        .map(|line| {
            if line.starts_with(".tuple-new ") {
                let mut fields = line.split_whitespace().collect::<Vec<_>>();
                fields.swap(2, 3);
                fields.join(" ")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(parse_unit_ir(&changed).is_err());
}

#[test]
fn selection_requires_its_member_api_and_delegate_types_do_not_become_views() {
    let source = "class D extends WatchUi.BehaviorDelegate { function initialize() { BehaviorDelegate.initialize(); } function onSelect() { return false; } }";
    let unit = compile_sources(&[source]).unwrap();
    let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 1, 0)).unwrap();
    assert!(
        unit.validate_api(context)
            .unwrap_err()
            .to_string()
            .contains("onSelect")
    );
    let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 2, 0)).unwrap();
    unit.validate_api(context).unwrap();
    for extra in [
        "function f() as D { return new D(); }",
        "function f(view as Toybox.WatchUi.View) {} function g() { f(new D()); }",
    ] {
        assert!(compile_sources(&[source, extra]).is_err());
    }
    let source = "function f(delegate as Toybox.WatchUi.BehaviorDelegate) as Toybox.WatchUi.BehaviorDelegate { return delegate; }";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn delegate_factories_and_native_aliases_preserve_the_native_base() {
    let app = APP.replace("new CounterDelegate(view)", "Delegates.make(view)");
    let factory = "module Delegates { function make(view as CounterView) as Toybox.WatchUi.BehaviorDelegate { return new CounterDelegate(view); } }";
    let unit = compile_sources(&[&app, factory, DELEGATE, VIEW]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    for changed in [
        DELEGATE
            .replace("using Toybox.WatchUi;", "import Toybox.WatchUi;")
            .replace(
                "extends WatchUi.BehaviorDelegate",
                "extends BehaviorDelegate",
            ),
        DELEGATE
            .replace("using Toybox.WatchUi;", "using Toybox.WatchUi as Ui;")
            .replace("WatchUi.BehaviorDelegate", "Ui.BehaviorDelegate")
            .replace(
                "BehaviorDelegate.initialize()",
                "Ui.BehaviorDelegate.initialize()",
            )
            .replace("WatchUi.requestUpdate()", "Ui.requestUpdate()"),
        DELEGATE.replace("return true", "return false"),
    ] {
        let unit = compile_sources(&[APP, &changed, VIEW]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
}
