use ciq_compiler::{
    ParameterType, ReturnType,
    api::{ApiContext, ApiLevel, Type},
    application::{ConsoleApplication, build_console_application},
    compile_function, parse_function_ir, parse_unit_ir,
};

const PROGRAM: &str = include_str!("../../../examples/ir-any/program.ciqir");

#[test]
fn any_contracts_preserve_in_memory_text_and_complete_application_stages() {
    let unit = parse_unit_ir(PROGRAM).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    let options = ConsoleApplication {
        id: [61; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    let app = build_console_application(&unit, &options).unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(app, build_console_application(&resumed, &options).unwrap());
    assert_eq!(ParameterType::Any.api_type(), Type::Any);
    assert_eq!(ReturnType::Any.api_type(), Type::Any);
    assert_eq!(Type::Any.with_null(), Type::Any);
    ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0))
        .unwrap()
        .check_type(ciq_compiler::api::core(), &Type::Any)
        .unwrap();
}

#[test]
fn dynamic_operands_use_shared_operations_without_accepting_known_type_errors() {
    for body in [
        ".parameter %0 0\n.not %1 %0\n.return %1",
        ".parameter %0 0\n.negate %1 %0\n.return %1",
        ".parameter %0 0\n.complement %1 %0\n.return %1",
        ".parameter %0 0\n.increment %1 %0\n.return %1",
        ".parameter %0 0\n.integer %1 2\n.equal %2 %0 %1\n.return %2",
        ".parameter %0 0\n.local $0 %0\n.string %1 \"next\"\n.assign $0 %1\n.read %2 $0\n.return %2",
    ] {
        let text = format!(
            "ciq-ir\n.function f parameters=1 parameter-types=any result=any\n{body}\n.end\n"
        );
        let f = parse_function_ir(&text).unwrap_or_else(|e| panic!("{text}: {e}"));
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        assert_eq!(
            f.lower(),
            ciq_assembly::Program::parse(&f.assembly()).unwrap()
        );
    }
    for source in [
        "function f(){return true-1;}",
        "function f(){return \"x\".missing();}",
        "function f(n as Toybox.Lang.Number){return n.length();}",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>){a[0]=\"bad\";return 0;}",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
}

#[test]
fn dynamic_metadata_preserves_structural_validation_and_explicit_void() {
    for body in [
        ".parameter %0 1\n.return %0",
        ".parameter %0 0\n.call-native %1 %0 bad.name\n.return %1",
        ".return-void",
        ".variable $0\n.read-variable %0 $0 any\n.return %0",
        ".parameter %0 0\n.variable $0 %0\n.read-variable %1 $0 integer\n.return %1",
    ] {
        let text = format!(
            "ciq-ir\n.function f parameters=1 parameter-types=any result=any\n{body}\n.end\n"
        );
        assert!(parse_function_ir(&text).is_err(), "{text}");
    }
    let text = "ciq-ir\n.function f result=void\n.null %0\n.return %0\n.end\n";
    assert!(parse_function_ir(text).is_err());
}

#[test]
fn dynamic_boundaries_retain_known_arity_availability_and_source_contracts() {
    for body in [
        ".string %0 \"x\"\n.parameter %1 0\n.call-native %2 %0 length %1\n.return %2",
        ".parameter %0 0\n.call-native %1 %0 length\n.return %1",
    ] {
        let text = format!(
            "ciq-ir\n.function f parameters=1 parameter-types=any result=any\n{body}\n.end\n"
        );
        let result = parse_function_ir(&text);
        if body.starts_with(".string") {
            assert!(
                result.is_err(),
                "known length still requires zero arguments"
            );
        } else {
            assert!(result.is_ok(), "dynamic length defers its receiver type");
        }
    }
    let text = "ciq-unit\n.owner global\nciq-ir\n.function f parameters=1 parameter-types=any result=source(C@Lang.Object)\n.parameter %0 0\n.return %0\n.end\n.class C Lang.Object\nciq-ir\n.function initialize receiver=instance\n.return-void\n.end\n.end-class\n";
    let unit = parse_unit_ir(text).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let low = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let text = "ciq-ir\n.function f parameters=1 parameter-types=any result=any\n.integer %0 42\n.parameter %1 0\n.call-native %2 %0 toChar %1\n.return %2\n.end\n";
    assert!(ciq_compiler::parse_function_ir_with_api(text, low).is_err());
}

#[test]
fn omitted_source_annotations_are_dynamic_and_saved_stages_preserve_them() {
    for source in [
        "function f(x){return x;}",
        "function f(x){return x.length();}",
        "function f(x,y){return x+y;}",
        "function f(x,key){return x[key];}",
        "function f(x,key,v){x[key]=v;return x[key];}",
        "function f(x){x=\"next\";return x;}",
        "function f(n){var count=n;var total=0;while(count>0){total+=count;count-=1;}return total;}",
        "function f(n){var count=n;var total=0;while(count>0){var a=[count,(true||false).hashCode()];total+=a[0];count-=1;}return total;}",
        "function f(){return \"hi\";}",
        "function f(){return null;}",
        "function f(b){if(b){return 42;}else{return \"no\";}}",
        "function f(b){if(b){return 42;}else{return null;}}",
        "function f(){}",
        "function f(){return;}",
        "function f(b){if(b){return;}else{System.println(1);}}",
    ] {
        let f = compile_function(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f.return_type(), ReturnType::Any, "{source}");
        assert!(
            f.parameter_types()
                .iter()
                .all(|ty| *ty == ParameterType::Any)
        );
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        assert_eq!(
            f.lower(),
            ciq_assembly::Program::parse(&f.assembly()).unwrap()
        );
    }
    for source in [
        "function f(b){if(b){return 42;}}",
        "function f(b){if(b){return 42;}else{return;}}",
        "function f() as Void{return 42;}",
        "function f() as Toybox.Lang.Number{return null;}",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    let unit =
        ciq_compiler::compile_sources(&[include_str!("../../../examples/ir-any/app.mc")]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let app = ConsoleApplication {
        id: [62; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    build_console_application(&unit, &app).unwrap();
}

#[test]
fn concrete_source_results_accept_dynamic_values_without_losing_saved_contracts() {
    for ty in [
        "Number",
        "Long",
        "Float",
        "Double",
        "String",
        "Boolean",
        "Object",
        "Array<Toybox.Lang.Number>",
    ] {
        let source = format!("function f(x) as Toybox.Lang.{ty}{{return x;}}");
        let f = compile_function(&source).unwrap();
        assert!(f.to_ir_text().contains("result="));
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap(), "{source}");
    }
}

#[test]
fn dynamic_lengths_and_elements_preserve_checked_container_contracts() {
    for source in [
        "function f(n) as Toybox.Lang.ByteArray{return new[n]b;}",
        "function f(n) as Toybox.Lang.Array<Toybox.Lang.Number or Null>{return new[n];}",
        "function f(x){var a=[x,7];a[0]=null;return a[0];}",
        "function f(x){var a=[x];a.add(7);return a[1];}",
    ] {
        let f = compile_function(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    let f = compile_function("function f(x){return [x,7];}").unwrap();
    assert!(f.to_ir_text().contains(".tuple-new"));
    for text in [
        "ciq-ir\n.function f\n.array-constant %0 any 0x00000001\n.return %0\n.end\n",
        "ciq-unit\n.owner global\nciq-ir\n.function use parameters=1 parameter-types=number-array\n.integer %0 0\n.return %0\n.end\n.owner global\nciq-ir\n.function f\n.array-new %0 any\n.call-global %1 use %0\n.return %1\n.end\n",
    ] {
        if text.starts_with("ciq-unit") {
            // An unknown element contract may cross a gradual typed boundary.
            assert!(parse_unit_ir(text).is_ok());
        } else {
            assert!(parse_function_ir(text).is_err());
        }
    }
}
