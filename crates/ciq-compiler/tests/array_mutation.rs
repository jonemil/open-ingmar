use ciq_compiler::ArrayElement;
use ciq_compiler::{
    Function, Operation, ParameterType, ReturnType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_function_with_api, compile_sources_with_api, parse_function_ir,
    parse_function_ir_with_api, parse_unit_ir_with_api, symbol_bindings,
};

fn context() -> ApiContext {
    ApiContext::new(ApiLevel(1, 3, 0), ApiLevel(1, 3, 0)).unwrap()
}

#[test]
fn array_mutation_preserves_source_saved_ir_and_application_pipelines() {
    for source in [
        include_str!("../../../examples/compiler-array-mutation/app.mc"),
        include_str!("../../../examples/compiler-array-add/app.mc"),
        include_str!("../../../examples/compiler-array-transforms/app.mc"),
    ] {
        let unit = compile_sources_with_api(&[source], context()).unwrap();
        let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context()).unwrap();
        assert_eq!(unit, resumed);
        let options = ConsoleApplication {
            id: [41; 16],
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
    }
    let symbols = ciq_assembly::Program::parse(
        ".symbol addAll \"addAll\" id=0x008004be\n\
         .symbol remove \"remove\" id=0x008000b7\n\
         .symbol removeAll \"removeAll\" id=0x008004c0",
    )
    .unwrap();
    for (member, result, argument, id) in [
        ("addAll", "Array<Toybox.Lang.Char>", "a", 0x008004be),
        ("remove", "Boolean", "'a'", 0x008000b7),
        ("removeAll", "Boolean", "'a'", 0x008004c0),
    ] {
        let source = format!(
            "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Toybox.Lang.{result} {{ return a.{member}({argument}); }}"
        );
        let f = compile_function_with_api(&source, context()).unwrap();
        assert_eq!(
            f,
            parse_function_ir_with_api(&f.to_ir_text(), context()).unwrap()
        );
        let assembly = f.lower();
        let bindings = symbol_bindings(&assembly, &symbols).unwrap();
        assert_eq!(bindings[&format!("__ciq_native_{member}")], id);
        assert!(
            assembly
                .encode_with_references(&bindings, &Default::default())
                .is_ok()
        );
    }
}

#[test]
fn generic_mutation_arguments_and_results_are_rechecked_in_saved_ir() {
    for (member, argument) in [
        ("addAll", "null"),
        ("addAll", "\"ab\""),
        ("addAll", "'a'"),
        ("remove", "65"),
        ("remove", "null"),
        ("removeAll", "true"),
        ("removeAll", "\"a\""),
    ] {
        // Use a numeric consumer so a bad argument cannot hide behind an
        // unannotated array/Boolean return restriction.
        let source = format!(
            "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) {{ return a.{member}({argument}).hashCode(); }}"
        );
        let error = compile_function(&source).unwrap_err();
        assert!(error.message.contains("argument types"), "{error:?}");
    }
    for member in ["addAll", "remove", "removeAll"] {
        for args in ["", "'a', 'b'"] {
            let source = format!(
                "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) {{ return a.{member}({args}).size(); }}"
            );
            assert!(
                compile_function(&source)
                    .unwrap_err()
                    .message
                    .contains("argument count")
            );
        }
    }
    let source = "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>, b as Toybox.Lang.Array<Toybox.Lang.Char>) as Toybox.Lang.Array<Toybox.Lang.Char> { return a.addAll(b); }";
    let f = compile_function_with_api(source, context()).unwrap();
    let text = f.to_ir_text();
    assert!(
        parse_function_ir(&text.replace("char-array:char-array", "char-array:string")).is_err()
    );
    assert!(parse_function_ir(&text.replace("addAll", "remove")).is_err());
    let old = ApiContext::new(ApiLevel(1, 2, 0), ApiLevel(1, 2, 0)).unwrap();
    assert!(compile_function_with_api(source, old).is_err());
    assert!(parse_function_ir_with_api(&text, old).is_err());
    let f = compile_function("function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Toybox.Lang.Boolean { return !a.removeAll('a'); }").unwrap();
    assert_eq!(f.return_type(), ReturnType::Boolean);
    assert!(parse_function_ir(&f.to_ir_text().replace("removeAll", "add")).is_err());
}

#[test]
fn saved_ir_cannot_move_size_before_a_mutation_consumed_first() {
    let f = compile_function("function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) { return a.remove('a').hashCode() + a.size(); }").unwrap();
    let mut operations = f.operations().to_vec();
    let mutation = operations
        .iter()
        .position(|op| {
            matches!(
                &op.operation, Operation::NativeCall { member, .. } if member == "remove"
            )
        })
        .unwrap();
    let size = operations
        .iter()
        .position(|op| {
            matches!(
                &op.operation, Operation::NativeCall { member, .. } if member == "size"
            )
        })
        .unwrap();
    let moved = operations.drain(size - 1..=size).collect::<Vec<_>>();
    operations.splice(mutation..mutation, moved);
    let error = Function::with_signature(
        "bad".into(),
        vec![ParameterType::Array(ArrayElement::Char)],
        ReturnType::Integer,
        operations,
    )
    .unwrap_err();
    assert!(error.message.contains("evaluation order"), "{error:?}");
}
