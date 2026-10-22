use ciq_assembly::Program;
use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_function_with_api, compile_sources, compile_sources_with_api,
    parse_function_ir, parse_function_ir_with_api, parse_unit_ir,
};

fn context() -> ApiContext {
    ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap()
}

const ORDER: &str = "class Order {function initialize() {} function compare(a as Lang.Object,b as Lang.Object) as Lang.Number {return 0;}}";

#[test]
fn object_sort_preserves_complete_saved_stages() {
    let unit = compile_sources_with_api(
        &[include_str!(
            "../../../examples/compiler-object-sort/app.mc"
        )],
        context(),
    )
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [100; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
    let assembly = unit.lower();
    assert_eq!(assembly, Program::parse(&assembly.to_text()).unwrap());
    assert!(assembly.to_text().contains("getsv @__ciq_native_sort"));
}

#[test]
fn custom_sort_accepts_all_supported_non_null_reference_elements() {
    for element in [
        "Object",
        "Symbol",
        "Method",
        "WeakReference",
        "ByteArray",
        "Comparable",
        "Comparator",
        "Toybox.WatchUi.View",
    ] {
        let element = if element.starts_with("Toybox.") {
            element.into()
        } else {
            format!("Lang.{element}")
        };
        let source =
            format!("function f(a as Lang.Array<{element}>, c as Order) as Void {{a.sort(c);}}");
        let unit = compile_sources_with_api(&[&source, ORDER], context())
            .unwrap_or_else(|e| panic!("{source}: {e:?}"));
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
        let native = format!(
            "function f(a as Lang.Array<{element}>, c as Lang.Comparator) as Void {{a.sort(c);}}"
        );
        let f = compile_function_with_api(&native, context()).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        let broken = ORDER.replace("a as Lang.Object", "a as Lang.Number");
        assert!(compile_sources(&[&source, &broken]).is_err());
        if element != "Lang.Object" && element != "Lang.Comparable" {
            for comparator in ["null", "c"] {
                let source = format!(
                    "function f(a as Lang.Array<{element}>, c as Lang.Comparator or Null) as Void {{a.sort({comparator});}}"
                );
                assert!(compile_function(&source).is_err(), "{source}");
            }
        }
    }
}

#[test]
fn dynamic_object_ordering_keeps_api_checks_and_runtime_domains() {
    let source = "function f(a as Lang.Array<Lang.Object>) as Void {a.sort(null);}";
    let f = compile_function_with_api(source, context()).unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    let old = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(4, 2, 0)).unwrap();
    assert!(compile_function_with_api(source, old).is_err());
    assert!(parse_function_ir_with_api(&f.to_ir_text(), old).is_err());
    // Statically erased element domains cannot promise runtime comparability.
    assert!(compile_function("function f() as Void {var a = [\"1\", 1]; a.sort(null);}").is_err());
    let unit = compile_sources(&["module M {function values() as Lang.Array<Lang.Object> {return [\"1\",1];} function f() as Void {M.values().sort(null);}}"]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    for element in [
        "Object",
        "Number",
        "Numeric",
        "ByteArray",
        "Symbol",
        "Method",
        "WeakReference",
        "Comparable",
        "Comparator",
    ] {
        for argument in ["null", "c"] {
            let source = format!(
                "function f(a as Lang.Array<Lang.{element} or Null>, c as Lang.Comparator) as Void {{a.sort({argument});}}"
            );
            assert!(compile_function(&source).is_err(), "{source}");
        }
    }
    let source =
        "function f(a as Lang.Array<Lang.ByteArray>,c as Lang.Comparator) as Void {a.sort(c);}";
    let text = compile_function(source).unwrap().to_ir_text();
    assert!(parse_function_ir(&text.replace(":comparator", ":nullable-comparator")).is_err());
    assert!(
        parse_function_ir(&text.replace("byte-array-array", "nullable-byte-array-array")).is_err()
    );
}

#[test]
fn sorting_preserves_callback_shutdown_and_initialization_checks() {
    let target = "class Target {function initialize(){}}";
    for call in ["a.sort(null);", "Helpers.sort(a);"] {
        let source = format!(
            "module Helpers {{function sort(a as Lang.Array<Lang.Object>) as Void {{a.sort(null);}}}} class Holder {{var target as Target; function initialize(a as Lang.Array<Lang.Object>,t as Target) {{{call} self.target=t;}}}}"
        );
        let error = compile_sources(&[&source, target]).unwrap_err().to_string();
        assert!(error.contains("shutdown"), "{error}");
        let after = source.replace(
            &format!("{call} self.target=t;"),
            &format!("self.target=t; {call}"),
        );
        assert!(compile_sources(&[&after, target]).is_ok());
    }
    // A pinned comparator with a known safe body does not gain erased default
    // callbacks merely because its receiver is an Object Array.
    let safe = "class Holder {var target as Target; function initialize(a as Lang.Array<Lang.Object>,c as Order,t as Target) {a.sort(c); self.target=t;}}";
    assert!(compile_sources(&[safe, ORDER, target]).is_ok());
    let exiting = ORDER.replace("return 0;", "System.exit(); return 0;");
    assert!(
        compile_sources(&[safe, &exiting, target])
            .unwrap_err()
            .to_string()
            .contains("shutdown")
    );
}
