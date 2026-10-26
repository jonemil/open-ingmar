#[path = "common/checking.rs"]
mod checking;

use ciq_compiler::ArrayElement;
use ciq_compiler::{
    Function, Operation, ParameterType, ReturnType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_function_with_api, compile_sources_with_api, parse_function_ir,
    parse_function_ir_with_api, parse_unit_ir_with_api,
};
use ciq_mc::Opcode;

fn context() -> ApiContext {
    ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 3, 0)).unwrap()
}

#[test]
fn native_statements_preserve_complete_pipelines_and_statement_spans() {
    let unit = compile_sources_with_api(
        &[include_str!(
            "../../../examples/compiler-native-statements/app.mc"
        )],
        context(),
    )
    .unwrap();
    let resumed = parse_unit_ir_with_api(&unit.to_ir_text(), context()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [42; 16],
        entry: "start".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap(),
    );
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    let source = "function f(n as Toybox.Lang.Number) as Void { n.abs(); }";
    let f = compile_function(source).unwrap();
    assert_eq!(f.return_type(), ReturnType::Void);
    let discard = f
        .operations()
        .iter()
        .find(|op| matches!(op.operation, Operation::Discard { .. }))
        .unwrap();
    let span = discard.span.unwrap();
    assert_eq!(&source[span.start..span.end], "n.abs();");
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
}

#[test]
fn native_statements_pop_one_final_result_for_zero_argument_and_chained_calls() {
    for (source, last_invoke) in [
        (
            "function f(n as Toybox.Lang.Number) as Void { n.abs(); }",
            Opcode::Invokemz,
        ),
        (
            "function f(n as Toybox.Lang.Number) as Void { n.toString(); }",
            Opcode::Invokemz,
        ),
        (
            "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Void { a.remove('a'); }",
            Opcode::Invokem,
        ),
        (
            "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Void { a.add('a').size(); }",
            Opcode::Invokemz,
        ),
        (
            "function f(s as Toybox.Lang.String) as Void { s.find(\"z\"); }",
            Opcode::Invokem,
        ),
    ] {
        let f = compile_function(source).unwrap();
        let assembly = f.lower();
        let opcodes = assembly
            .items()
            .iter()
            .filter_map(|item| match item {
                ciq_assembly::Item::Instruction { opcode, .. } => Some(*opcode),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            &opcodes[opcodes.len() - 3..],
            &[last_invoke, Opcode::Popv, Opcode::Return]
        );
        assert_eq!(
            opcodes
                .iter()
                .filter(|opcode| **opcode == Opcode::Popv)
                .count(),
            1
        );
    }
}

#[test]
fn discarding_a_result_does_not_bypass_native_contract_or_null_proof_checks() {
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Void { a.remove(65); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Void { a.addAll(null); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Void { a.sort(1); }",
        "function f(n as Toybox.Lang.Number) as Void { n.abs(1); }",
        "function f(s as Toybox.Lang.String or Null) as Void { s.length(); }",
        "function f() as Void { 1 + 2; }",
        "function f() as Void { \"abc\".toCharArray()[0]; }",
    ] {
        assert!(checking::compile_function(source).is_err(), "{source}");
    }
    let f = compile_function(
        "function f(s as Toybox.Lang.String or Null) as Void { if (!(s == null)) { s.length(); } }",
    )
    .unwrap();
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    let source = "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Void { a.remove('a'); }";
    let f = compile_function_with_api(source, context()).unwrap();
    let old = ApiContext::new(ApiLevel(1, 2, 0), ApiLevel(1, 2, 0)).unwrap();
    assert!(compile_function_with_api(source, old).is_err());
    assert!(parse_function_ir_with_api(&f.to_ir_text(), old).is_err());
    assert!(parse_function_ir(&f.to_ir_text().replace("char-array", "string")).is_err());
    for literal in [".integer %0 1", ".null %0", ".boolean %0 false"] {
        let text = format!("ciq-ir\n.function f\n{literal}\n.discard %0\n.return-void\n.end\n");
        let f = parse_function_ir(&text).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        assert!(parse_function_ir(&text.replace(".discard %0", ".discard %999")).is_err());
        assert!(
            parse_function_ir(&text.replace(".discard %0", ".discard %0\n.discard %0")).is_err()
        );
        assert!(parse_function_ir(&text.replace(".discard %0", ".discard %0 %1")).is_err());
    }
}

#[test]
fn saved_ir_cannot_defer_a_discarded_mutation_past_a_later_read() {
    let f = compile_function(
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Void { a.remove('a'); a.size(); }",
    )
    .unwrap();
    let mut ops = f.operations().to_vec();
    let mutation = ops.iter().position(|op| matches!(&op.operation, Operation::NativeCall { member, .. } if member == "remove")).unwrap();
    let size = ops
        .iter()
        .position(
            |op| matches!(&op.operation, Operation::NativeCall { member, .. } if member == "size"),
        )
        .unwrap();
    let moved = ops.drain(size - 1..=size).collect::<Vec<_>>();
    ops.splice(mutation..mutation, moved);
    let error = Function::with_signature(
        "bad".into(),
        vec![ParameterType::Array(ArrayElement::Char)],
        ReturnType::Void,
        ops,
    )
    .unwrap_err();
    assert!(error.message.contains("evaluation order"), "{error:?}");
}
