use ciq_assembly::{Item, Program};
use ciq_compiler::{
    ClassBase, ClassDeclaration, CompilationUnit, Function, FunctionDeclaration, LocatedOperation,
    Operation, ReturnType, ValueId,
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, parse_ir, parse_unit_ir,
};
use ciq_mc::{Opcode, decode};
use std::{fs, process::Command};

fn op(operation: Operation) -> LocatedOperation {
    LocatedOperation {
        operation,
        span: None,
    }
}
fn example() -> CompilationUnit {
    compile_sources(&[
        include_str!("../../../examples/compiler-class/app.mc"),
        include_str!("../../../examples/compiler-class/numbers.mc"),
    ])
    .unwrap()
}
fn app(entry: &str) -> ConsoleApplication {
    ConsoleApplication {
        id: [3; 16],
        entry: entry.into(),
    }
}

#[test]
fn class_ownership_receiver_and_text_pipeline_preserve_complete_prg() {
    let unit = example();
    assert_eq!(unit.classes().len(), 1);
    let class = &unit.classes()[0];
    assert_eq!(class.name, "Counter");
    assert_eq!(class.base, ClassBase::ApplicationAppBase);
    assert!(class.methods.iter().all(Function::is_instance_method));
    assert_eq!(class.methods[0].return_type(), ReturnType::Void);
    assert!(unit.to_ir_text().starts_with("ciq-unit\n"));
    assert!(
        unit.to_ir_text()
            .contains(".class Counter Application.AppBase\n")
    );
    assert!(class.methods[0].to_ir_text().contains("receiver=instance"));
    assert_eq!(parse_unit_ir(&unit.to_ir_text()).unwrap(), unit);
    let typed =
        CompilationUnit::with_classes(unit.declarations().to_vec(), unit.classes().to_vec())
            .unwrap();
    assert_eq!(typed, unit);
    let native = build_console_application(&unit, &app("Counter.start")).unwrap();
    let resumed = build_console_application(
        &parse_unit_ir(&unit.to_ir_text()).unwrap(),
        &app("Counter.start"),
    )
    .unwrap();
    assert_eq!(native.to_bytes(), resumed.to_bytes());
    let assembly = unit.lower();
    assert_eq!(
        assembly.items(),
        Program::parse(&assembly.to_text()).unwrap().items()
    );
    let labels = assembly.labels().unwrap();
    assert_ne!(
        labels[&class.code_label("answer")],
        labels["__ciq_fn_g_6_answer"]
    );
    assert_ne!(
        labels[&class.code_label("answer")],
        labels["__ciq_fn_m_7_Numbers_6_answer"]
    );
    let code = native
        .sections()
        .iter()
        .find(|s| s.kind() == ciq_prg::CODE)
        .unwrap();
    let decoded = decode(code.payload()).unwrap();
    let start = decoded.iter().position(|i| i.offset == 0x10).unwrap();
    assert_eq!(decoded[start + 1].instruction.opcode(), Opcode::Getselfv);
    assert_eq!(decoded[start + 2].instruction.opcode(), Opcode::Invokemz);
}

#[test]
fn self_calls_resolve_only_within_the_enclosing_class_and_check_signatures() {
    for source in [
        "class C extends Application.AppBase { function start() { self.missing(); } }",
        "class C extends Application.AppBase { function start() { self.f(); } function f(x) {} }",
        "class C extends Application.AppBase { function start() { return self.f(); } function f() {} }",
        "class C extends Application.AppBase { function start() { return M.f(); } function f() { return 1; } }",
        "class C extends Application.AppBase { function start() { C.f(); } function f() {} }",
        "module M { function f() {} } class C extends Application.AppBase { function start() { self.f(); } }",
        "function f() { return 1; } class C extends Application.AppBase { function start() { return self.f(); } }",
        "class C extends Application.AppBase { function f() { D.f(); } } class D extends Application.AppBase { function f() {} }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let unit = compile_sources(&["class C extends Application.AppBase { function f(x) { return self.g(x); } function g(x) { return x; } } class D extends Application.AppBase { function f() { self.f(); } }"]).unwrap();
    assert_eq!(unit.classes().len(), 2);
    assert_eq!(
        unit.classes()[0].methods[0].return_type(),
        ReturnType::Integer
    );
    assert_eq!(parse_unit_ir(&unit.to_ir_text()).unwrap(), unit);
    assert!(
        build_console_application(&unit, &app("C.f"))
            .unwrap_err()
            .message
            .contains("exactly one")
    );
    let error = compile_sources(&[
        "function main() {}",
        "class C extends Application.AppBase {\n function f() { self.missing(); }\n}",
    ])
    .unwrap_err();
    assert_eq!(error.source, 1);
    assert_eq!(error.diagnostic.line, 2);
    assert!(error.diagnostic.message.contains("C.missing"));
}

#[test]
fn receiver_contract_cannot_be_invented_by_global_module_or_text_producers() {
    let operations = vec![
        op(Operation::SelfCallStatement {
            member: "f".into(),
            arguments: vec![],
        }),
        op(Operation::ReturnVoid),
    ];
    assert!(Function::new("f".into(), operations.clone()).is_err());
    let method = Function::method("f".into(), 0, operations).unwrap();
    assert_eq!(parse_ir(&method.to_ir_text()).unwrap(), method);
    assert!(
        CompilationUnit::new(vec![FunctionDeclaration {
            module: None,
            function: method.clone()
        }])
        .is_err()
    );
    assert!(
        CompilationUnit::new(vec![FunctionDeclaration {
            module: Some("M".into()),
            function: method
        }])
        .is_err()
    );
    let free = compile("function f() {}").unwrap();
    assert!(
        CompilationUnit::with_classes(
            vec![],
            vec![ClassDeclaration {
                name: "C".into(),
                base: ClassBase::ApplicationAppBase,
                fields: vec![],
                methods: vec![free]
            }]
        )
        .is_err()
    );
    for source in [
        "function f() { self.f(); }",
        "function f() { return self.f(); }",
        "function f(self) {}",
        "function f() { var self = 1; }",
        "function f() { return self; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    for text in [
        "ciq-ir\n.function f\n.call-self-discard f\n.return-void\n.end",
        "ciq-ir\n.function f receiver=instance receiver=instance\n.return-void\n.end",
        "ciq-ir\n.function f receiver=unknown\n.return-void\n.end",
        "ciq-ir\n.function f receiver=instance\n.call-self %0\n.return %0\n.end",
        "ciq-ir\n.function f receiver=instance\n.call-self-discard\n.return-void\n.end",
    ] {
        assert!(parse_ir(text).is_err(), "{text}");
    }
}

#[test]
fn class_syntax_names_and_container_framing_reject_unsupported_inputs() {
    for source in [
        "class C { function f() {} }",
        "class C extends Other { function f() {} }",
        "class C extends Application.AppBase {}",
        "class C extends Application.AppBase { var x = 1; }",
        "class C extends Application.AppBase { function f() {} function f() {} }",
        "class C extends Application.AppBase { function f() {} } class C extends Application.AppBase { function g() {} }",
        "module C { function f() {} } class C extends Application.AppBase { function g() {} }",
        "function C() {} class C extends Application.AppBase { function f() {} }",
        "class System extends Application.AppBase { function f() {} }",
        "class AppBase extends Application.AppBase { function f() {} }",
        "class println extends Application.AppBase { function f() {} }",
        "module M { class C extends Application.AppBase { function f() {} } }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let text = example().to_ir_text();
    for bad in [
        text.replace(".end-class", ""),
        text.replace("Application.AppBase", "Other"),
        text.replace("receiver=instance", ""),
    ] {
        assert!(parse_unit_ir(&bad).is_err());
    }
    let plain = compile_sources(&["function main() {}"]).unwrap();
    assert!(plain.to_ir_text().starts_with("ciq-unit\n"));
    assert_eq!(parse_unit_ir(&plain.to_ir_text()).unwrap(), plain);
    let too_many = format!(
        "class C extends Application.AppBase {{ {} }}",
        (0..257)
            .map(|i| format!("function f{i}() {{}}"))
            .collect::<String>()
    );
    assert!(
        compile_sources(&[&too_many])
            .unwrap_err()
            .diagnostic
            .message
            .contains("256-function")
    );
}

#[test]
fn self_calls_use_receiver_lookup_and_preserve_nested_argument_order() {
    let method = Function::method(
        "f".into(),
        0,
        vec![
            op(Operation::SelfCall {
                result: ValueId(0),
                result_type: ciq_compiler::ReturnType::Integer,
                member: "left".into(),
                arguments: vec![],
            }),
            op(Operation::SelfCall {
                result: ValueId(1),
                result_type: ciq_compiler::ReturnType::Integer,
                member: "right".into(),
                arguments: vec![],
            }),
            op(Operation::SelfCallStatement {
                member: "consume".into(),
                arguments: vec![ValueId(0), ValueId(1)],
            }),
            op(Operation::ReturnVoid),
        ],
    )
    .unwrap();
    let instructions: Vec<_> = method
        .lower()
        .items()
        .iter()
        .filter_map(|i| {
            if let Item::Instruction { opcode, .. } = i {
                Some(*opcode)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        instructions,
        [
            Opcode::Argc,
            Opcode::Getselfv,
            Opcode::Frpush,
            Opcode::Getselfv,
            Opcode::Invokemz,
            Opcode::Getselfv,
            Opcode::Invokemz,
            Opcode::Invokem,
            Opcode::Popv,
            Opcode::Return
        ]
    );
    let text = method
        .to_ir_text()
        .replace("consume %0 %1", "consume %1 %0");
    assert!(
        parse_ir(&text)
            .unwrap_err()
            .message
            .contains("evaluation order")
    );
}

#[test]
fn class_application_validates_entry_and_metadata_conflicts() {
    let unit = example();
    for entry in [
        "start",
        "Other.start",
        "Counter.missing",
        "Counter.answer",
        "answer",
    ] {
        assert!(
            build_console_application(&unit, &app(entry)).is_err(),
            "{entry}"
        );
    }
    assert!(compile_sources(&[
        "class C extends Application.AppBase { function start() {} function getInitialView() {} }",
    ]).is_err());
    let unit =
        compile_sources(&["class C extends Application.AppBase { function C() { return 41; } }"])
            .unwrap();
    let prg = build_console_application(&unit, &app("C.C")).unwrap();
    let decoded = decode(
        prg.sections()
            .iter()
            .find(|s| s.kind() == ciq_prg::CODE)
            .unwrap()
            .payload(),
    )
    .unwrap();
    let calls: Vec<_> = decoded
        .iter()
        .filter(|i| {
            i.instruction.opcode() == Opcode::Getselfv && i.instruction.operands() == [0, 0, 0, 1]
        })
        .collect();
    assert_eq!(calls.len(), 1);
}

#[test]
fn class_cli_roundtrips_ir_and_rejects_invalid_receiver_without_output() {
    let dir = std::env::temp_dir().join(format!("ciq-class-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let source = dir.join("app.mc");
    let modules = dir.join("numbers.mc");
    let ir = dir.join("app.ciqir");
    fs::write(
        &source,
        include_str!("../../../examples/compiler-class/app.mc"),
    )
    .unwrap();
    fs::write(
        &modules,
        include_str!("../../../examples/compiler-class/numbers.mc"),
    )
    .unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
        .args(["--unit", "--emit", "ir"])
        .arg(&source)
        .arg(&modules)
        .arg(&ir)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let direct = dir.join("direct.prg");
    let resumed = dir.join("resumed.prg");
    for (from, inputs, output) in [
        ("source", vec![&source, &modules], &direct),
        ("ir", vec![&ir], &resumed),
    ] {
        let run = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
            .args([
                "--unit",
                "--from",
                from,
                "--emit",
                "prg",
                "--app-id",
                "3c94c9e5ba2c43fa849d64ecff33264e",
                "--entry",
                "Counter.start",
            ])
            .args(inputs)
            .arg(output)
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
    assert_eq!(fs::read(&direct).unwrap(), fs::read(&resumed).unwrap());
    fs::write(
        &source,
        "class Counter extends Application.AppBase { function start() { self.missing(); } }",
    )
    .unwrap();
    let invalid = dir.join("invalid.prg");
    let run = Command::new(env!("CARGO_BIN_EXE_ciq-compiler"))
        .args([
            "--unit",
            "--emit",
            "prg",
            "--app-id",
            "3c94c9e5ba2c43fa849d64ecff33264e",
            "--entry",
            "Counter.start",
        ])
        .arg(&source)
        .arg(&invalid)
        .output()
        .unwrap();
    assert!(!run.status.success());
    assert!(!invalid.exists());
    assert!(String::from_utf8_lossy(&run.stderr).contains("Counter.missing"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn generated_class_fields_bind_own_methods_and_keep_same_name_owners_distinct() {
    use ciq_prg::data::classes::{Record, records};
    let unit = example();
    let prg = build_console_application(&unit, &app("Counter.start")).unwrap();
    let records = records(prg.unique_section(ciq_prg::DATA).unwrap().payload()).unwrap();
    let classes: Vec<_> = records
        .iter()
        .filter_map(|r| {
            if let Record::Class(c) = r {
                Some(c)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(classes.len(), 4);
    let global = classes[1];
    let class = classes[2];
    let module = classes[3];
    assert!(
        classes
            .iter()
            .all(|c| c.fields.windows(2).all(|w| w[0].key < w[1].key))
    );
    assert_eq!(class.extends, Some(0x40000000));
    assert_eq!(class.parent, Some(0x008002e6));
    assert_eq!(
        global
            .fields
            .iter()
            .find(|f| f.symbol_id() == 1)
            .unwrap()
            .value as usize,
        class.offset
    );
    let label = unit.lower().labels().unwrap();
    // The void class bootstrap is 73 bytes in this fixed profile.
    let origin = 73;
    for method in &unit.classes()[0].methods {
        let pc = origin + label[&unit.classes()[0].code_label(method.name())];
        assert_eq!(
            class
                .fields
                .iter()
                .filter(|f| f.code_offset() == Some(pc))
                .count(),
            1
        );
    }
    let global_answer = global
        .fields
        .iter()
        .find(|f| f.code_offset() == Some(origin + label["__ciq_fn_g_6_answer"]))
        .unwrap();
    let module_answer = module
        .fields
        .iter()
        .find(|f| f.code_offset() == Some(origin + label["__ciq_fn_m_7_Numbers_6_answer"]))
        .unwrap();
    let class_answer = class
        .fields
        .iter()
        .find(|f| f.code_offset() == Some(origin + label[&unit.classes()[0].code_label("answer")]))
        .unwrap();
    assert_eq!(global_answer.symbol_id(), class_answer.symbol_id());
    assert_eq!(module_answer.symbol_id(), class_answer.symbol_id());
    assert_ne!(global_answer.value, class_answer.value);
    assert_ne!(module_answer.value, class_answer.value);
}
