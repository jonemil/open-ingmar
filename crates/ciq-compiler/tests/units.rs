use ciq_assembly::Program;
use ciq_compiler::{
    CompilationUnit, FunctionDeclaration, compile_function, compile_sources, parse_unit_ir,
    symbol_bindings,
};
use std::{fs, process::Command};

fn sources() -> [&'static str; 2] {
    [
        include_str!("../../../examples/compiler-unit/entry.mc"),
        include_str!("../../../examples/compiler-unit/numbers.mc"),
    ]
}
fn symbols() -> Program {
    Program::parse(".symbol g \"globals\" id=0x008002e6\n.symbol m \"globals/Numbers\" id=0x02\n.symbol d \"difference\" id=0x80\n.symbol l \"left\" id=0x81\n.symbol r \"right\" id=0x82\n.symbol s \"Toybox_System\" id=0x83\n.symbol p \"println\" id=0x84\n").unwrap()
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
        .replace("ciq-unit", "ciq-unit # header")
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
    let callee = "module Numbers { function difference(a as Toybox.Lang.Number,b as Toybox.Lang.Number) as Toybox.Lang.Number { return a - b; } }";
    for (caller, message) in [
        (
            "function f() as Toybox.Lang.Number { return Numbers.difference(1); }",
            "expects 2 arguments, got 1",
        ),
        (
            "function f() as Toybox.Lang.Number { return Numbers.difference(1,2,3); }",
            "expects 2 arguments, got 3",
        ),
        (
            "function f() as Toybox.Lang.Number { return Numbers.unknown(); }",
            "unresolved function",
        ),
        (
            "function f() as Toybox.Lang.Number { return Missing.difference(1,2); }",
            "unresolved function",
        ),
        (
            "function f() as Toybox.Lang.Number { return Numbers.difference(\"x\",2); }",
            "argument type does not match parameter contract",
        ),
        (
            "function f() as Toybox.Lang.Number { return Numbers.difference(1 == 1,2); }",
            "argument type does not match parameter contract",
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
    let caller = "\nfunction f() as Toybox.Lang.Number {\n    return Numbers.difference();\n}";
    let error = compile_sources(&[callee, caller]).unwrap_err();
    assert_eq!(
        (error.source, error.diagnostic.line, error.diagnostic.column),
        (1, 3, 12)
    );
    // Single-function compilation retains its intentionally external callee mode.
    assert!(
        compile_function("function f() as Toybox.Lang.Number { return Missing.external(); }")
            .is_ok()
    );
}

#[test]
fn duplicate_and_conflicting_declarations_fail_for_source_typed_and_text_units() {
    let source = "function f() as Toybox.Lang.Number { return 1; }";
    let error = compile_sources(&[source, source]).unwrap_err();
    assert_eq!(error.source, 1);
    assert!(error.diagnostic.message.contains("duplicate"));
    for source in [
        "module A { function f() as Toybox.Lang.Number { return 1; } function f() as Toybox.Lang.Number { return 2; } }",
        "module System { function f() as Toybox.Lang.Number { return 1; } }",
        "function A() as Toybox.Lang.Number { return 1; } module A { function f() as Toybox.Lang.Number { return 1; } }",
        "module A {}",
        "module A { module B {} }",
        "function f() as Toybox.Lang.Number { return 1; } garbage",
        "function f() as Toybox.Lang.Number { return \"x\"; }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let declaration = FunctionDeclaration {
        module: Some("A".into()),
        function: compile_function("function f() as Toybox.Lang.Number { return 1; }").unwrap(),
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
    assert!(parse_unit_ir(&(text.clone() + &text["ciq-unit\n".len()..])).is_err());
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
        "module A { function first(n as Toybox.Lang.Number) as Toybox.Lang.Number { if (n == 0) { return 0; } return A.second(n - 1); } }",
        "module A { function second(n as Toybox.Lang.Number) as Toybox.Lang.Number { if (n == 0) { return 1; } return A.first(n - 1); } }",
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
    let caller =
        compile_function("function f() as Toybox.Lang.Number { return A.second(); }").unwrap();
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
    let function = "ciq-ir\n.function f\n.integer %0 1\n.return %0\n.end\n";
    for text in [
        "".to_owned(),
        "ciq-unit unexpected\n".to_owned(),
        "ciq-unit\n".to_owned(),
        format!("ciq-unit\n{function}"),
        format!("ciq-unit\n.owner global extra\n{function}"),
        format!(
            "ciq-unit\n.owner module A\n{}",
            function.replace(".end\n", "")
        ),
        format!("ciq-unit\n.owner global\n{function}trailing\n"),
    ] {
        assert!(parse_unit_ir(&text).is_err(), "{text}");
    }
    let error = parse_unit_ir(&format!(
        "ciq-unit\n.owner global\n{}",
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
    let extra = "function extra() as Toybox.Lang.Number { return 1; }";
    assert!(compile_sources(&[&source, extra]).is_err());
    assert!(compile_sources(&[&(source + extra)]).is_err());
    let text = unit.to_ir_text()
        + ".owner global\nciq-ir\n.function extra\n.integer %0 1\n.return %0\n.end\n";
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

#[test]
fn owner_relative_calls_keep_signatures_context_and_text_pipelines() {
    let source = r#"
        module M {
            function sum(n as Toybox.Lang.Number) as Toybox.Lang.Number { if(n == 0) { return 0; } return n + sum(n - 1); }
            function step(n as Toybox.Lang.Number) as Toybox.Lang.Number { return n + 1; }
            function count(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { return a.size(); }
            function notify() as Void { System.println(1); }
            function run(step as Toybox.Lang.Number) as Toybox.Lang.Number { var M = 7; notify(); step(1); return step(M) + sum(3) + count([]); }
        }
        class Token {
            function initialize() {}
            function sum(n as Toybox.Lang.Number) as Toybox.Lang.Number { if(n == 0) { return 0; } return n + sum(n - 1); }
            function count(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { return a.size(); }
            function notify() as Void { System.println(1); }
            function run(sum as Toybox.Lang.Number) as Toybox.Lang.Number { notify(); sum(1); return sum(3) + count([]); }
        }
    "#;
    let unit = ciq_compiler::compile_sources(&[source]).unwrap();
    assert_eq!(
        unit,
        ciq_compiler::parse_unit_ir(&unit.to_ir_text()).unwrap()
    );
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    assert!(unit.declarations().iter().flat_map(|decl| decl.function.operations()).any(|op|
        matches!(&op.operation, ciq_compiler::Operation::ModuleCallStatement { module, member, .. }
            if module == "M" && member == "notify")));
    let first = "module First { function run() as Toybox.Lang.Number { return later(4); } function later(n as Toybox.Lang.Number) as Toybox.Lang.Number { return n; } }";
    let other = "module Other { function later(n as Toybox.Lang.Number) as Toybox.Lang.String { return \"other\"; } }";
    let unit = ciq_compiler::compile_sources(&[first, other]).unwrap();
    assert_eq!(
        unit,
        ciq_compiler::parse_unit_ir(&unit.to_ir_text()).unwrap()
    );
}

#[test]
fn owner_relative_calls_reject_unresolved_targets_and_invalid_consumers() {
    for source in [
        "module M { function f() as Toybox.Lang.Number { return missing(); } }",
        "class C { function initialize(){} function f() as Toybox.Lang.Number { return missing(); } }",
        "module M { function f() as Toybox.Lang.Number { return target(); } function target(n as Toybox.Lang.Number) as Toybox.Lang.Number { return n; } }",
        "module M { function f() as Toybox.Lang.Number { return target(true); } function target(n as Toybox.Lang.Number) as Toybox.Lang.Number { return n; } }",
        "module M { function f() as Toybox.Lang.Number { return target(); } function target() as Void {} }",
        "module M { function f() as Toybox.Lang.Number { var value=1; return value(); } }",
        "function f() as Toybox.Lang.Number{return $.missing();}",
    ] {
        assert!(
            ciq_compiler::compile_sources(&[source]).is_err(),
            "{source}"
        );
    }
}

#[test]
fn global_calls_keep_owner_identity_signatures_and_saved_stages() {
    let first = r#"
        function step(n as Toybox.Lang.Number) as Toybox.Lang.Number { return n + 1; }
        function total(n as Toybox.Lang.Number) as Toybox.Lang.Number { if(n==0) { return 0; } return step(n) + total(n-1); }
        function count(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { return a.size(); }
        function notify() as Void { System.println(1); }
        function title() as Toybox.Lang.String { return "x"; }
        function token() as C { return new C(); }
    "#;
    let second = r#"
        module M {
            function step(n as Toybox.Lang.Number) as Toybox.Lang.Number { return n * 2; }
            function run() as Toybox.Lang.Number { notify(); $.notify(); step(1); $.step(1); return total(3) + $.step(4) + count([]) + title().length() + token().run(); }
        }
        class C {
            function initialize() {}
            function step(n as Toybox.Lang.Number) as Toybox.Lang.Number { return n * 3; }
            function run() as Toybox.Lang.Number { notify(); return total(3) + $.step(4) + step(4); }
        }
    "#;
    let unit = compile_sources(&[first, second]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let assembly = unit.lower();
    assert_eq!(assembly, Program::parse(&assembly.to_text()).unwrap());
    assert!(
        assembly
            .to_text()
            .contains("getmv @__ciq_source_globals @__ciq_member_step")
    );
    let text = unit.to_ir_text();
    assert!(text.contains(".call-global "));
    assert!(text.contains(".call-global-discard notify"));
    assert!(parse_unit_ir(&text.replace("title result=string", "title")).is_err());
    assert!(
        parse_unit_ir(&text.replace(
            ".call-global-discard notify",
            ".call-global-discard missing"
        ))
        .is_err()
    );
    assert!(compile_sources(&["function f() as Toybox.Lang.Number { return f(); }"]).is_ok());
    for source in [
        "function g(n as Toybox.Lang.Number) as Toybox.Lang.Number { return n; } function f() as Toybox.Lang.Number { return g(); }",
        "function g(n as Toybox.Lang.Number) as Toybox.Lang.Number { return n; } function f() as Toybox.Lang.Number { return g(true); }",
        "function g() as Void {} function f() as Toybox.Lang.Number { return g(); }",
        "function g() as Toybox.Lang.Number { return 1; } module M { function g(n as Toybox.Lang.Number) as Toybox.Lang.Number{return n;} function f() as Toybox.Lang.Number{return g();} }",
        "function f() as Toybox.Lang.Number{return $.missing();}",
        "function g() as Toybox.Lang.Number{return 1;} class C { var g=1; function f() as Toybox.Lang.Number{return g();} }",
        "function f() as Toybox.Lang.Number{return $f();}",
        "function f() as Toybox.Lang.Number{return $.f;}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    for text in [
        "ciq-ir\n.function f\n.call-global\n.end\n",
        "ciq-ir\n.function f\n.call-global-discard\n.return-void\n.end\n",
        "ciq-ir\n.function f\n.call-global %0 bad-name\n.return %0\n.end\n",
    ] {
        assert!(ciq_compiler::parse_function_ir(text).is_err(), "{text}");
    }
}

#[test]
fn root_module_calls_bypass_lexical_qualifiers_and_keep_contracts() {
    let module = r#"module M {
        function step(n as Toybox.Lang.Number) as Toybox.Lang.Number { return n + 1; }
        function notify() as Void { System.println(1); }
        function count(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { return a.size(); }
    }"#;
    let callers = r#"
        function run(M as Toybox.Lang.Number) as Toybox.Lang.Number { $.M.notify(); $.M.step(1); return $.M.step(M) + $.M.count([]); }
        module Other { function run() as Toybox.Lang.Number { var M = 7; return $.M.step(M); } }
        class C {
            function initialize() {}
            function run(M as Toybox.Lang.Number) as Toybox.Lang.Number { $.M.notify(); return $.M.step(M); }
        }
    "#;
    let unit = compile_sources(&[module, callers]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let assembly = unit.lower();
    assert_eq!(assembly, Program::parse(&assembly.to_text()).unwrap());
    assert!(
        assembly
            .to_text()
            .contains("getmv @__ciq_module_M @__ciq_member_step")
    );
    for caller in [
        "function f() as Toybox.Lang.Number{return $.M.missing();}",
        "function f() as Toybox.Lang.Number{return $.M.step();}",
        "function f() as Toybox.Lang.Number{return $.M.step(true);}",
        "function f() as Toybox.Lang.Number{return $.M.notify();}",
        "function f() as Toybox.Lang.Number{return $.M;}",
        "function f() as Toybox.Lang.Number{return $.M.step;}",
        "function f() as Toybox.Lang.Number{return $.M.N.step(1);}",
        "function f() as Toybox.Lang.Number{return $.M.();}",
    ] {
        assert!(compile_sources(&[module, caller]).is_err(), "{caller}");
    }
    assert!(
        compile_sources(&[
            "class C { function initialize(){} function step() as Toybox.Lang.Number{return 1;} } function f() as Toybox.Lang.Number{return $.C.step();}"
        ])
        .is_err()
    );
    for depth in [64, 65] {
        let caller = format!(
            "function f() as Toybox.Lang.Number {{ return {}1{}; }}",
            "$.M.step(".repeat(depth),
            ")".repeat(depth)
        );
        let result = compile_sources(&[module, &caller]);
        if depth == 64 {
            result.unwrap();
        } else {
            assert!(result.unwrap_err().diagnostic.message.contains("nesting"));
        }
    }
}
