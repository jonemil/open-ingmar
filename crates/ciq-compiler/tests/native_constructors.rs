use ciq_compiler::{ClassBase, Operation, compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn direct_object_construction_preserves_the_native_contract_at_api_1() {
    use ciq_compiler::{
        ReturnType,
        api::{ApiContext, ApiLevel},
        compile_sources_with_api, parse_unit_ir_with_api,
    };

    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    for (imports, target) in [
        ("using Toybox.Lang;", "Lang.Object"),
        ("", "Toybox.Lang.Object"),
        ("using Toybox.Lang as L;", "L.Object"),
        ("import Toybox.Lang;", "Object"),
    ] {
        let source =
            format!("{imports} function make() as Toybox.Lang.Object {{ return new {target}(); }}");
        let unit = compile_sources_with_api(&[&source], initial).unwrap();
        assert_eq!(
            unit.declarations()[0].function.return_type(),
            ReturnType::LangObject
        );
        let text = unit.to_ir_text();
        assert!(text.contains(".new-native %0 Toybox.Lang.Object"), "{text}");
        assert_eq!(parse_unit_ir_with_api(&text, initial).unwrap(), unit);
    }
    for source in [
        "function make() { return new Toybox.Lang.Object(1); }",
        "using Toybox.Lang as L; function make(L) { return new L.Object(); }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}

#[test]
fn source_allocation_checks_the_class_and_explicit_native_calls_check_the_method_api() {
    use ciq_compiler::{
        api::{ApiContext, ApiLevel},
        compile_sources_with_api, parse_unit_ir_with_api,
    };
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let source = "class C extends Toybox.WatchUi.View{}function make(){return new C();}";
    let unit = compile_sources_with_api(&[source], initial).unwrap();
    assert_eq!(
        parse_unit_ir_with_api(&unit.to_ir_text(), initial).unwrap(),
        unit
    );
    let explicit = source.replace("View{}", "View{function initialize(){View.initialize();}}");
    assert!(compile_sources_with_api(&[&explicit], initial).is_err());
    let later = ApiContext::new(ApiLevel(2, 1, 0), ApiLevel(2, 1, 0)).unwrap();
    let unit = compile_sources_with_api(&[&explicit], later).unwrap();
    assert!(parse_unit_ir_with_api(&unit.to_ir_text(), initial).is_err());
}

#[test]
fn native_rooted_construction_preserves_complete_saved_application() {
    let unit = checking::compile_sources(&[include_str!(
        "../../../examples/compiler-native-constructors/app.mc"
    )])
    .unwrap();
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: [14; 16],
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    for name in [
        "EmptyView",
        "EmptyDelegate",
        "DefaultView",
        "DefaultDelegate",
    ] {
        let class = unit.classes().iter().find(|c| c.name == name).unwrap();
        let initialize = class
            .methods
            .iter()
            .find(|f| f.name() == "initialize")
            .unwrap();
        assert_eq!(initialize.parameter_count(), 0);
        assert!(
            initialize
                .operations()
                .iter()
                .all(|op| matches!(op.operation, Operation::ReturnVoid))
        );
    }
    let text = unit.to_ir_text();
    let bad = text.replace(
        ".base-initialize WatchUi.BehaviorDelegate",
        ".base-initialize WatchUi.View",
    );
    assert_ne!(bad, text);
    assert!(parse_unit_ir(&bad).is_err());
}

#[test]
fn native_base_initialization_is_optional_and_keeps_ordinary_statement_order() {
    for base in ["View", "BehaviorDelegate"] {
        for body in [
            "",
            "BASE.initialize();",
            "Toybox.System.println(1);BASE.initialize();",
            "BASE.initialize();BASE.initialize();",
            "if(true){BASE.initialize();}",
        ] {
            let body = body.replace("BASE", base);
            let source = format!(
                "class C extends Toybox.WatchUi.{base}{{function initialize(){{{body}}}}}function make(){{return new C();}}"
            );
            common::pipeline(compile_sources(&[&source]).unwrap());
        }
        let wrong = if base == "View" {
            "BehaviorDelegate"
        } else {
            "View"
        };
        for body in [
            format!("{wrong}.initialize();"),
            format!("{base}.initialize(1);"),
            format!("var x={base}.initialize();"),
        ] {
            let source =
                format!("class C extends Toybox.WatchUi.{base}{{function initialize(){{{body}}}}}");
            assert!(compile_sources(&[&source]).is_err(), "{source}");
        }
    }
}

#[test]
fn native_defaults_and_constructor_requirements_resolve_across_files() {
    for (base, kind) in [
        ("View", ClassBase::WatchUiView),
        ("BehaviorDelegate", ClassBase::WatchUiBehaviorDelegate),
    ] {
        let root = format!(
            "module M{{class Root extends Toybox.WatchUi.{base}{{var n as Toybox.Lang.Number;function initialize(n as Toybox.Lang.Number){{self.n=n;}}}}}}"
        );
        let leaf = "using M as T;class Leaf extends T.Root{function initialize(){T.Root.initialize(3);}}function make(){return new Leaf();}";
        for sources in [vec![root.as_str(), leaf], vec![leaf, root.as_str()]] {
            let unit = common::pipeline(checking::compile_sources(&sources).unwrap());
            assert!(unit.classes().iter().all(|c| c.base == kind));
        }
        assert!(
            compile_sources(&[
                &root,
                "class Leaf extends M.Root{}function make(){return new Leaf();}"
            ])
            .is_err()
        );
        let defaults = root.replace(
            "var n as Toybox.Lang.Number;",
            "var n as Toybox.Lang.Number=10;",
        );
        common::pipeline(
            compile_sources(&[
                &defaults,
                "class Leaf extends M.Root{}function make(){return new Leaf();}",
            ])
            .unwrap(),
        );
        assert!(
            compile_sources(&[
                &defaults,
                "class Leaf extends M.Root{}function make(){return new Leaf(1);}"
            ])
            .is_err()
        );
        let unsafe_hook = root.replace("var n as Toybox.Lang.Number;", "var n=self;");
        assert!(compile_sources(&[&unsafe_hook, leaf]).is_err());
    }
}

#[test]
fn native_calls_do_not_bypass_required_fields_or_initializer_scope() {
    for base in ["View", "BehaviorDelegate"] {
        for source in [
            format!(
                "class C extends Toybox.WatchUi.{base}{{var n as Toybox.Lang.Number;}}function make(){{return new C();}}"
            ),
            format!(
                "class C extends Toybox.WatchUi.{base}{{var n as Toybox.Lang.Number;function initialize(){{if(true){{{base}.initialize();}}n=1;}}}}"
            ),
            format!(
                "class C extends Toybox.WatchUi.{base}{{function other(){{{base}.initialize();}}}}"
            ),
        ] {
            assert!(compile_sources(&[&source]).is_err(), "{source}");
        }
    }
}
