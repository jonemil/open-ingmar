use ciq_assembly::{Item, Program};
use ciq_compiler::{
    Function, LocatedOperation, Operation, ReturnType, ValueId,
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources, parse_function_ir, parse_unit_ir,
};
use ciq_mc::{Opcode, decode};
use std::{fs, process::Command};

fn located(operation: Operation) -> LocatedOperation {
    LocatedOperation {
        operation,
        span: None,
    }
}

#[test]
fn nonreturning_functions_preserve_declared_results_through_saved_ir() {
    for (annotation, tag) in [
        ("Number", "integer"),
        ("Long", "long"),
        ("Float", "float"),
        ("Double", "double"),
        ("String", "string"),
        ("Boolean", "boolean"),
        ("Char", "char"),
        ("Number or Null", "nullable-integer"),
        ("Array<Toybox.Lang.Number>", "number-array"),
        ("Object", "object"),
        ("Symbol", "symbol"),
    ] {
        let source = format!("function never() as Toybox.Lang.{annotation} {{for(;;){{}}}}");
        let function = compile_function(&source).unwrap();
        let text = function.to_ir_text();
        assert!(text.contains(&format!(" result={tag}\n")), "{text}");
        assert_eq!(function, parse_function_ir(&text).unwrap());
        assert_eq!(
            function.lower(),
            Program::parse(&function.assembly()).unwrap()
        );
        // Losing the contract changes the signature, even though no value
        // returns can expose that loss inside this body.
        assert_ne!(
            function.return_type(),
            parse_function_ir(&text.replace(&format!(" result={tag}"), ""))
                .unwrap()
                .return_type()
        );
    }
    for source in [
        "function f() as Toybox.Lang.Number {for(;;){break;}}",
        "function f() as Toybox.Lang.Number {for(;;){return;}}",
        "function f() as Toybox.Lang.Number {while(true){}}",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    assert!(parse_function_ir("ciq-ir\n.function f result=integer\n.return-void\n.end\n").is_err());
}

#[test]
fn nonreturning_source_results_keep_unit_call_and_class_checks() {
    let source = "class Token {function initialize(){}} module M {function never() as Token {for(;;){}} function forward() as Token {return M.never();} function discard() as Void {M.never();}}";
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    assert_eq!(
        unit.lower(),
        Program::parse(&unit.lower().to_text()).unwrap()
    );
    let invalid = text.replace(
        ".function never result=source(Token@Lang.Object)",
        ".function never result=integer",
    );
    assert_ne!(invalid, text);
    assert!(parse_unit_ir(&invalid).is_err());
    let unresolved = "ciq-unit\n.owner global\nciq-ir\n.function never result=source(Missing@Lang.Object)\n.block ^0\n.jump ^0\n.block ^1\n.return-void\n.end\n";
    assert!(parse_unit_ir(unresolved).is_err());
    let source = "using Toybox.Application; class App extends Application.AppBase {function initialize(){AppBase.initialize();} function getInitialView(){for(;;){}}}";
    let unit = compile_sources(&[source]).unwrap();
    assert!(unit.to_ir_text().contains("result=one-view"));
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn empty_explicit_and_falling_through_functions_have_void_returns() {
    for source in [
        "function f() as Void {}",
        "function f() as Void { return; }",
        "function f() as Void { System.println(41); }",
        "function f(x as Toybox.Lang.Number) as Void { if (x == 0) {} else {} }",
        "function f(x as Toybox.Lang.Number) as Void { if (x == 0) {} else { if (x > 1) {} else {} } }",
        "function f(x as Toybox.Lang.Number) as Void { while (x > 0) { continue; } }",
        "function f(x as Toybox.Lang.Number) as Void { while (x > 0) { break; } }",
        "function f(x as Toybox.Lang.Number) as Void { if (x == 0) { return; } System.println(x); }",
        "function f(x as Toybox.Lang.Number) as Void { while (x > 0) { return; } }",
        "function f(x as Toybox.Lang.Number) as Void { if (x == 0) { return; } else { return; } }",
    ] {
        let function = compile_function(source).unwrap();
        assert_eq!(function.return_type(), ReturnType::Void, "{source}");
        assert!(function.to_ir_text().starts_with("ciq-ir\n"));
        assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
        let assembly = function.lower();
        assert_eq!(
            assembly.items(),
            Program::parse(&assembly.to_text()).unwrap().items()
        );
        assert_eq!(
            function.operations().last().unwrap().operation,
            Operation::ReturnVoid
        );
    }
    let empty = Function::new("empty".into(), vec![located(Operation::ReturnVoid)]).unwrap();
    let instructions = decode(&empty.lower().encode().unwrap()).unwrap();
    assert_eq!(instructions.len(), 2);
    assert_eq!(instructions[0].instruction.opcode(), Opcode::Argc);
    assert_eq!(instructions[1].instruction.opcode(), Opcode::Return);
    let explicit = compile_function("function f() as Void { return; }").unwrap();
    let span = explicit.operations()[0].span.unwrap();
    assert_eq!(
        &"function f() as Void { return; }"[span.start..span.end],
        "return;"
    );
}

#[test]
fn mixed_returns_unreachable_statements_and_invalid_void_ir_are_rejected() {
    for source in [
        "function f(x as Toybox.Lang.Number) as Toybox.Lang.Number { if (x == 0) { return; } return 1; }",
        "function f(x as Toybox.Lang.Number) as Toybox.Lang.Number { if (x == 0) { return 1; } return; }",
        "function f(x as Toybox.Lang.Number) as Toybox.Lang.Number { if (x == 0) { return 1; } }",
        "function f(x as Toybox.Lang.Number) as Toybox.Lang.Number { while (x > 0) { return 1; } }",
        "function f() as Void { return; System.println(1); }",
        "function f(x as Toybox.Lang.Number) as Void { if (x == 0) { return; } else { return; } return; }",
        "function f() as Void { var M = 1; M.run(); }",
        "function f(M as Toybox.Lang.Number) as Void { M.run(); }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    for body in [
        ".return-void\n.return-void",
        ".integer %0 1\n.return-void",
        ".call-discard M run %0\n.return-void",
        ".call-discard M\n.return-void",
        ".return-void %0",
        ".block ^0\n.integer %0 1\n.integer %1 1\n.equal %2 %0 %1\n.branch %2 ^1 ^2\n.block ^1\n.return-void\n.block ^2\n.integer %3 1\n.return %3",
        ".integer %0 1\n.integer %1 2\n.call-discard M run %1 %0\n.return-void",
    ] {
        assert!(
            parse_function_ir(&format!("ciq-ir\n.function f\n{body}\n.end")).is_err(),
            "{body}"
        );
    }
    assert!(Function::new("f".into(), vec![]).is_err());
    assert!(
        Function::new(
            "f".into(),
            vec![located(Operation::Integer {
                result: ValueId(0),
                value: 1
            })]
        )
        .is_err()
    );
}

#[test]
fn unit_signatures_reject_void_values_and_check_statement_call_arity() {
    let prefix = "module M { function run(x as Toybox.Lang.Number) as Void { System.println(x); } function value() as Toybox.Lang.Number { return 1; } }";
    for body in [
        "return M.run(1);",
        "var x = M.run(1); return x;",
        "M.run();",
        "M.run(1, 2);",
        "M.missing();",
        "M.run(M.run(1));",
    ] {
        assert!(
            compile_sources(&[&format!("function f() as Void {{ {body} }} {prefix}")]).is_err(),
            "{body}"
        );
    }
    // Signature collection checks forward calls and recursive void references.
    let unit = compile_sources(&[&format!(
        "function f() as Void {{ M.run(M.value()); M.value(); }} {prefix}"
    )])
    .unwrap();
    assert_eq!(parse_unit_ir(&unit.to_ir_text()).unwrap(), unit);
    let recursive = compile_sources(&["module M { function run() as Void { M.run(); } }"]).unwrap();
    assert_eq!(
        recursive.declarations()[0].function.return_type(),
        ReturnType::Void
    );
    let bad = unit
        .to_ir_text()
        .replace(".call-discard M run %0", ".call %1 M run %0\n.println %1");
    assert!(parse_unit_ir(&bad).is_err());
}

#[test]
fn statement_calls_preserve_argument_order_and_discard_results() {
    let function =
        compile_function("function f() as Void { M.run(M.left(), M.right()); M.empty(); return; }")
            .unwrap();
    let text = function.to_ir_text();
    assert!(text.contains(".call-discard M run %0 %1"));
    assert!(text.contains(".call-discard M empty"));
    assert_eq!(parse_function_ir(&text).unwrap(), function);
    let assembly = function.lower();
    let instructions: Vec<_> = assembly
        .items()
        .iter()
        .filter_map(|i| {
            if let Item::Instruction { opcode, operands } = i {
                Some((*opcode, operands))
            } else {
                None
            }
        })
        .collect();
    let opcodes: Vec<_> = instructions.iter().map(|(op, _)| *op).collect();
    assert_eq!(
        opcodes,
        [
            Opcode::Argc,
            Opcode::Getmv,
            Opcode::Frpush,
            Opcode::Getmv,
            Opcode::Invokemz,
            Opcode::Getmv,
            Opcode::Invokemz,
            Opcode::Invokem,
            Opcode::Popv,
            Opcode::Getmv,
            Opcode::Invokemz,
            Opcode::Popv,
            Opcode::Return
        ]
    );
    assert!(format!("{:?}", instructions[3].1).contains("left"));
    assert!(format!("{:?}", instructions[5].1).contains("right"));
}

#[test]
fn complete_void_application_has_saved_ir_parity_and_no_wrapper_print() {
    let sources = [
        include_str!("../../../examples/compiler-void/main.mc"),
        include_str!("../../../examples/compiler-void/workers.mc"),
    ];
    let unit = compile_sources(&sources).unwrap();
    let app = ConsoleApplication {
        id: [3; 16],
        entry: "main".into(),
    };
    let direct = build_console_application(&unit, &app).unwrap();
    let resumed =
        build_console_application(&parse_unit_ir(&unit.to_ir_text()).unwrap(), &app).unwrap();
    assert_eq!(direct.to_bytes(), resumed.to_bytes());
    let code = &direct
        .sections()
        .iter()
        .find(|s| s.kind() == ciq_prg::CODE)
        .unwrap()
        .payload();
    let decoded = decode(code).unwrap();
    // Synthetic onStart begins after bootstrap/noop/native constructor, at 0x10.
    let start = decoded.iter().position(|i| i.offset == 0x10).unwrap();
    assert_eq!(decoded[start].instruction.opcode(), Opcode::Argc);
    let startup: Vec<_> = decoded[start..]
        .iter()
        .take(5)
        .map(|i| i.instruction.opcode())
        .collect();
    assert_eq!(
        startup,
        [
            Opcode::Argc,
            Opcode::Getmv,
            Opcode::Invokemz,
            Opcode::Popv,
            Opcode::Return
        ]
    );
}

#[test]
fn cli_resumes_void_units_and_preserves_output_on_type_errors() {
    let dir = std::env::temp_dir().join(format!("ciq-compiler-void-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let main = dir.join("main.mc");
    let workers = dir.join("workers.mc");
    fs::write(
        &main,
        include_str!("../../../examples/compiler-void/main.mc"),
    )
    .unwrap();
    fs::write(
        &workers,
        include_str!("../../../examples/compiler-void/workers.mc"),
    )
    .unwrap();
    let ir = dir.join("unit.ciqir");
    let direct = dir.join("direct.prg");
    let resumed = dir.join("resumed.prg");
    let run = |args: Vec<&std::ffi::OsStr>| {
        let result = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
            .args(args)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    run(vec![
        "--unit".as_ref(),
        "--emit".as_ref(),
        "ir".as_ref(),
        main.as_os_str(),
        workers.as_os_str(),
        ir.as_os_str(),
    ]);
    for (from, inputs, output) in [
        (
            "source",
            vec![main.as_os_str(), workers.as_os_str()],
            &direct,
        ),
        ("ir", vec![ir.as_os_str()], &resumed),
    ] {
        let mut args = vec![
            "--unit".as_ref(),
            "--from".as_ref(),
            from.as_ref(),
            "--emit".as_ref(),
            "prg".as_ref(),
            "--app-id".as_ref(),
            "3c94c9e5ba2c43fa849d64ecff33264e".as_ref(),
        ];
        args.extend(inputs);
        args.push(output.as_os_str());
        run(args);
    }
    assert_eq!(fs::read(&direct).unwrap(), fs::read(&resumed).unwrap());
    fs::write(
        &main,
        "function main() as Toybox.Lang.Number { return Startup.begin(); }",
    )
    .unwrap();
    let output = dir.join("invalid.prg");
    let result = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
        .args([
            "--unit",
            "--emit",
            "prg",
            "--app-id",
            "3c94c9e5ba2c43fa849d64ecff33264e",
        ])
        .arg(&main)
        .arg(&workers)
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("void function"));
    assert!(!output.exists());
    fs::remove_dir_all(dir).unwrap();
}
