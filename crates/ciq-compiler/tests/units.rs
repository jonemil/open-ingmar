use ciq_assembly::Program;
use ciq_compiler::{
    CompilationUnit, FunctionDeclaration, compile, compile_sources, parse_unit_ir, symbol_bindings,
};
use std::{fs, process::Command};

fn sources() -> [&'static str; 2] {
    [
        include_str!("../../../examples/compiler-unit/entry.mc"),
        include_str!("../../../examples/compiler-unit/numbers.mc"),
    ]
}
fn symbols() -> Program {
    Program::parse(".symbol m \"globals/Numbers\" id=0x02\n.symbol d \"difference\" id=0x80\n.symbol l \"left\" id=0x81\n.symbol r \"right\" id=0x82\n.symbol s \"Toybox_System\" id=0x83\n.symbol p \"println\" id=0x84\n").unwrap()
}

#[test]
fn forward_calls_across_files_preserve_unit_ir_assembly_and_bytes() {
    let unit = compile_sources(&sources()).unwrap();
    assert_eq!(unit.declarations().len(), 4);
    assert_eq!(unit.declarations()[0].module, None);
    assert_eq!(unit.declarations()[1].module.as_deref(), Some("Numbers"));
    assert_eq!(unit.declarations()[1].function.parameter_count(), 2);
    assert_eq!(
        CompilationUnit::new(unit.declarations().to_vec()).unwrap(),
        unit
    );
    let assembly = unit.lower();
    let labels = assembly.labels().unwrap();
    let mut previous = None;
    for declaration in unit.declarations() {
        let offset = labels[&declaration.code_label()];
        assert!(previous.is_none_or(|p| p < offset));
        previous = Some(offset);
    }
    let decorated = unit
        .to_ir_text()
        .replace("ciq-unit 1", "ciq-unit 1 # header")
        .replace(".owner global", "\n# owner\n.owner global # entry")
        .replace(".end\n", ".end # function end\n")
        .replace('\n', "\r\n");
    assert_eq!(unit, parse_unit_ir(&decorated).unwrap());
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let bindings = symbol_bindings(&assembly, &symbols()).unwrap();
    let bytes = assembly
        .encode_with_references(&bindings, &Default::default())
        .unwrap();
    for model in [
        resumed.lower(),
        Program::parse(&assembly.to_text()).unwrap(),
    ] {
        assert_eq!(model, assembly);
        assert_eq!(
            model
                .encode_with_references(&bindings, &Default::default())
                .unwrap(),
            bytes
        );
    }
    let reordered = compile_sources(&[sources()[1], sources()[0]]).unwrap();
    assert_eq!(reordered.declarations()[0].function.name(), "difference");
    assert!(
        reordered
            .lower()
            .encode_with_references(&bindings, &Default::default())
            .is_ok()
    );
}

#[test]
fn source_unit_checks_callees_arity_types_and_file_locations() {
    let callee = "module Numbers { function difference(a,b) { return a - b; } }";
    for (caller, message) in [
        (
            "function f() { return Numbers.difference(1); }",
            "expects 2 arguments, got 1",
        ),
        (
            "function f() { return Numbers.difference(1,2,3); }",
            "expects 2 arguments, got 3",
        ),
        (
            "function f() { return Numbers.unknown(); }",
            "unresolved function",
        ),
        (
            "function f() { return Missing.difference(1,2); }",
            "unresolved function",
        ),
        (
            "function f() { return Numbers.difference(\"x\",2); }",
            "integer arguments",
        ),
        (
            "function f() { return Numbers.difference(1 == 1,2); }",
            "integer arguments",
        ),
    ] {
        let error = compile_sources(&[callee, caller]).unwrap_err();
        assert_eq!(error.source, 1);
        assert!(error.diagnostic.message.contains(message), "{error}");
        assert_eq!(
            error.diagnostic.offset,
            caller
                .find("Numbers.")
                .or_else(|| caller.find("Missing."))
                .unwrap()
        );
    }
    let caller = "\nfunction f() {\n    return Numbers.difference();\n}";
    let error = compile_sources(&[callee, caller]).unwrap_err();
    assert_eq!(
        (error.source, error.diagnostic.line, error.diagnostic.column),
        (1, 3, 12)
    );
    // Single-function compilation retains its intentionally external callee mode.
    assert!(compile("function f() { return Missing.external(); }").is_ok());
}

#[test]
fn duplicate_and_conflicting_declarations_fail_for_source_typed_and_text_units() {
    let source = "function f() { return 1; }";
    let error = compile_sources(&[source, source]).unwrap_err();
    assert_eq!(error.source, 1);
    assert!(error.diagnostic.message.contains("duplicate"));
    for source in [
        "module A { function f() { return 1; } function f() { return 2; } }",
        "module System { function f() { return 1; } }",
        "function A() { return 1; } module A { function f() { return 1; } }",
        "module A {}",
        "module A { module B {} }",
        "function f() { return 1; } garbage",
        "function f() { return \"x\"; }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let declaration = FunctionDeclaration {
        module: Some("A".into()),
        function: compile("function f() { return 1; }").unwrap(),
    };
    let duplicate =
        CompilationUnit::new(vec![declaration.clone(), declaration.clone()]).unwrap_err();
    assert_eq!(duplicate.declaration, Some(1));
    for module in ["System", "while", "bad.name", ""] {
        assert!(
            CompilationUnit::new(vec![FunctionDeclaration {
                module: Some(module.into()),
                ..declaration.clone()
            }])
            .is_err()
        );
    }
    let unit = CompilationUnit::new(vec![declaration]).unwrap();
    let text = unit.to_ir_text();
    assert!(parse_unit_ir(&(text.clone() + &text["ciq-unit 1\n".len()..])).is_err());
}

#[test]
fn ownership_mangling_prevents_member_internal_and_literal_alias_collisions() {
    let source = "function f() { var s = \"same\"; if (1 == 1) { return 1; } return 2; }
        module A { function f() { var s = \"same\"; if (1 == 1) { return 2; } return 3; } }
        module A_f { function x() { return 3; } }
        module A { function f_x() { return 4; } }";
    let unit = compile_sources(&[source]).unwrap();
    let assembly = unit.lower();
    let text = assembly.to_text();
    assert_eq!(unit.declarations()[0].code_label(), "__ciq_fn_g_1_f");
    assert_eq!(unit.declarations()[1].code_label(), "__ciq_fn_m_1_A_1_f");
    assert!(assembly.labels().is_ok());
    let aliases = assembly
        .items()
        .iter()
        .filter_map(|i| match i {
            ciq_assembly::Item::String(s) => Some(s.alias.clone()),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(aliases.len(), 2);
    assert_eq!(Program::parse(&text).unwrap(), assembly);
    assert_eq!(parse_unit_ir(&unit.to_ir_text()).unwrap(), unit);
}

#[test]
fn reopened_modules_and_recursive_calls_share_collected_signatures() {
    let unit = compile_sources(&[
        "module A { function first(n) { if (n == 0) { return 0; } return A.second(n - 1); } }",
        "module A { function second(n) { if (n == 0) { return 1; } return A.first(n - 1); } }",
    ])
    .unwrap();
    assert_eq!(parse_unit_ir(&unit.to_ir_text()).unwrap(), unit);
    let wrong = unit.to_ir_text().replace("A second", "A missing");
    assert!(
        parse_unit_ir(&wrong)
            .unwrap_err()
            .message
            .contains("unresolved")
    );
    let caller = compile("function f() { return A.second(); }").unwrap();
    let error = CompilationUnit::new(vec![
        FunctionDeclaration {
            module: None,
            function: caller,
        },
        unit.declarations()[1].clone(),
    ])
    .unwrap_err();
    assert_eq!(error.declaration, Some(0));
    assert!(error.operation.is_some());
    assert!(error.message.contains("expects 1 arguments"));
}

#[test]
fn unit_text_framing_and_combined_function_bounds_are_checked() {
    let function = "ciq-ir 1\n.function f\n.integer %0 1\n.return %0\n.end\n";
    for text in [
        "".to_owned(),
        "ciq-unit 3\n".to_owned(),
        "ciq-unit 1\n".to_owned(),
        format!("ciq-unit 1\n{function}"),
        format!("ciq-unit 1\n.owner global extra\n{function}"),
        format!(
            "ciq-unit 1\n.owner module A\n{}",
            function.replace(".end\n", "")
        ),
        format!("ciq-unit 1\n.owner global\n{function}trailing\n"),
    ] {
        assert!(parse_unit_ir(&text).is_err(), "{text}");
    }
    let error = parse_unit_ir(&format!(
        "ciq-unit 1\n.owner global\n{}",
        function.replace(".return %0", ".return %1")
    ))
    .unwrap_err();
    assert_eq!(error.line, 6);
    assert!(compile_sources(&[]).is_err());
    assert!(compile_sources(&["// empty"]).is_err());
    let source = (0..256)
        .map(|i| format!("function f{i}() {{ return 1; }}\n"))
        .collect::<String>();
    let unit = compile_sources(&[&source]).unwrap();
    assert_eq!(unit.declarations().len(), 256);
    let extra = "function extra() { return 1; }";
    assert!(compile_sources(&[&source, extra]).is_err());
    assert!(compile_sources(&[&(source + extra)]).is_err());
    let text = unit.to_ir_text()
        + ".owner global\nciq-ir 1\n.function extra\n.integer %0 1\n.return %0\n.end\n";
    assert!(parse_unit_ir(&text).is_err());
}

#[test]
fn cli_unit_sources_resume_ir_and_assembly_without_overwriting_outputs() {
    let root = std::env::temp_dir().join(format!("ciq-unit-test-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let entry = root.join("entry.mc");
    let members = root.join("members.mc");
    let ir = root.join("unit.ciqir");
    let assembly = root.join("unit.mcasm");
    let resumed = root.join("resumed.mcasm");
    let symbols_path = root.join("symbols.mcasm");
    let code = root.join("code.bin");
    let text_code = root.join("text.bin");
    fs::write(&entry, sources()[0]).unwrap();
    fs::write(&members, sources()[1]).unwrap();
    fs::write(&symbols_path, symbols().to_text()).unwrap();
    let executable = env!("CARGO_BIN_EXE_ciq-compiler");
    let output = Command::new(executable)
        .args(["--unit", "--emit", "ir"])
        .arg(&entry)
        .arg(&members)
        .arg(&ir)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    let output = Command::new(executable)
        .args(["--unit", "--from", "ir"])
        .arg(&ir)
        .arg(&assembly)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    let output = Command::new(executable)
        .args(["--unit"])
        .arg(&entry)
        .arg(&members)
        .arg(&resumed)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert_eq!(fs::read(&assembly).unwrap(), fs::read(&resumed).unwrap());
    for (from, input, target) in [("ir", &ir, &code), ("assembly", &assembly, &text_code)] {
        let output = Command::new(executable)
            .args(["--unit", "--from", from, "--emit", "code", "--symbols"])
            .arg(&symbols_path)
            .arg(input)
            .arg(target)
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output);
    }
    assert_eq!(fs::read(&code).unwrap(), fs::read(&text_code).unwrap());
    let output = Command::new(executable)
        .args(["--unit", "--from", "ir"])
        .arg(&ir)
        .arg(&assembly)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let output = Command::new(executable)
        .arg(&entry)
        .arg(&members)
        .arg(root.join("invalid"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    fs::remove_dir_all(root).unwrap();
}
