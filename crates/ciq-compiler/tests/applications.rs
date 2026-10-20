use ciq_compiler::{
    application::{ConsoleApplication, build_console_application},
    compile_sources, parse_unit_ir,
};
use ciq_prg::{
    Prg,
    data::classes::{self, Record},
};
use std::{collections::BTreeSet, fs, process::Command};

fn options(entry: &str) -> ConsoleApplication {
    ConsoleApplication {
        id: [0x41; 16],
        entry: entry.into(),
    }
}

#[test]
fn console_entry_results_follow_the_checked_println_domain() {
    for (ty, value) in [
        ("Number", "1"),
        ("Long", "1l"),
        ("Float", "1.5"),
        ("Double", "1.5d"),
        ("Numeric", "1"),
        ("Integer", "1l"),
        ("Decimal", "1.5"),
        ("String", "\"hello\""),
        ("Boolean", "true"),
        ("Char", "'a'"),
    ] {
        let source =
            format!("using Toybox.Lang as L; function main() as L.{ty} {{ return {value}; }}");
        let unit = compile_sources(&[&source]).unwrap();
        let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
        let direct = build_console_application(&unit, &options("main")).unwrap();
        assert_eq!(
            direct,
            build_console_application(&resumed, &options("main")).unwrap()
        );
    }
    for (ty, value) in [
        ("Object", "1"),
        ("Object or Null", "null"),
        ("Symbol", ":token"),
        ("ByteArray", "[]b"),
        ("ByteArray or Null", "null"),
        ("Method or Null", "null"),
        ("WeakReference", "new L.WeakReference()"),
        ("Numeric or Null", "null"),
        ("Integer or Null", "null"),
        ("Decimal or Null", "null"),
        ("Array<L.Number>", "[]"),
        ("Dictionary<L.String,L.Number>", "{}"),
    ] {
        let source =
            format!("using Toybox.Lang as L; function main() as L.{ty} {{ return {value}; }}");
        let unit = compile_sources(&[&source]).unwrap();
        let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
        for checked in [&unit, &resumed] {
            let error = build_console_application(checked, &options("main")).unwrap_err();
            assert!(
                error.to_string().contains("non-null scalar or Void"),
                "{ty}: {error}"
            );
        }
    }
}
fn word(bytes: &[u8]) -> u32 {
    u32::from_be_bytes(bytes.try_into().unwrap())
}
fn classes(prg: &Prg) -> Vec<classes::Class<'_>> {
    classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
        .unwrap()
        .into_iter()
        .filter_map(|r| {
            if let Record::Class(c) = r {
                Some(c)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn complete_application_resolves_classes_modules_methods_and_literal_data() {
    let unit = compile_sources(&[
        "function main() { System.println(\"hello é\\n\"); return A.f() + B.f(); }",
        "module A { function f() { var n = 2; while (n > 0) { n = n - 1; } return n; } } module B { function f() { return 42; } }",
    ]).unwrap();
    let app = build_console_application(&unit, &options("main")).unwrap();
    assert_eq!(Prg::parse(&app.to_bytes()).unwrap(), app);
    let records = classes(&app);
    assert_eq!(records.len(), 5);
    let statics = &records[0];
    let globals = &records[1];
    let application = &records[2];
    for c in &records {
        assert!(
            c.fields.windows(2).all(|w| w[0].key < w[1].key),
            "unsorted/duplicate runtime fields"
        );
    }
    let global_id = globals.module.unwrap();
    let global = statics
        .fields
        .iter()
        .find(|f| f.symbol_id() == global_id)
        .unwrap();
    assert_eq!(global.value as usize, globals.offset);
    let app_class = globals.fields.iter().find(|f| f.symbol_id() == 1).unwrap();
    assert_eq!(app_class.kind, 7);
    assert_eq!(app_class.value as usize, application.offset);
    assert_eq!(application.parent, Some(global_id));
    assert_eq!(application.extends, Some(0x40000000));
    for module in &records[3..] {
        assert_eq!(module.parent, Some(global_id));
        let id = module.module.unwrap();
        assert!(
            statics
                .fields
                .iter()
                .any(|f| f.symbol_id() == id && f.value as usize == module.offset)
        );
        assert!(globals.fields.iter().any(|f| f.kind == 10 && f.value == id));
    }
    let code = app.unique_section(ciq_prg::CODE).unwrap().payload();
    let instructions = ciq_mc::decode(code).unwrap();
    let boundaries: BTreeSet<_> = instructions.iter().map(|i| i.offset as u32).collect();
    let targets = records
        .iter()
        .flat_map(|c| c.fields.iter())
        .filter_map(|f| f.code_offset())
        .collect::<BTreeSet<_>>();
    assert!(targets.iter().all(|pc| boundaries.contains(pc)));
    // Same source member name in different modules binds distinct generated bodies.
    let a = records[3]
        .fields
        .iter()
        .find(|f| f.symbol_id() != 0x800018)
        .unwrap();
    let b = records[4]
        .fields
        .iter()
        .find(|f| f.symbol_id() != 0x800018)
        .unwrap();
    assert_eq!(a.symbol_id(), b.symbol_id());
    assert_ne!(a.value, b.value);
    let payload = app.unique_section(ciq_prg::DATA).unwrap().payload();
    let strings = classes::records(payload)
        .unwrap()
        .into_iter()
        .filter_map(|r| {
            if let Record::String { offset, value } = r {
                Some((offset, value))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(strings.len(), 1);
    assert_eq!(strings[0].1, "hello é\n");
    let news = instructions
        .iter()
        .find(|i| i.instruction.opcode() == ciq_mc::Opcode::News)
        .unwrap();
    assert_eq!(word(news.instruction.operands()) as usize, strings[0].0);
}

#[test]
fn source_typed_and_saved_unit_ir_produce_identical_complete_prgs() {
    let sources = [
        include_str!("../../../examples/compiler-unit/entry.mc"),
        include_str!("../../../examples/compiler-unit/numbers.mc"),
    ];
    let unit = compile_sources(&sources).unwrap();
    let app = build_console_application(&unit, &options("probe")).unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(
        app,
        build_console_application(&resumed, &options("probe")).unwrap()
    );
    let typed = ciq_compiler::CompilationUnit::new(unit.declarations().to_vec()).unwrap();
    assert_eq!(
        app,
        build_console_application(&typed, &options("probe")).unwrap()
    );
    assert_eq!(
        app,
        build_console_application(&unit, &options("probe")).unwrap()
    );
    let entry = app.unique_section(0x6060c0de).unwrap().payload();
    assert_eq!(&entry[2..18], &[0x41; 16]);
    let mut changed = options("probe");
    changed.id[0] = 0x42;
    let other = build_console_application(&unit, &changed).unwrap();
    for (a, b) in app.sections().iter().zip(other.sections()) {
        if a.kind() == 0x6060c0de {
            assert_ne!(a.payload(), b.payload());
        } else {
            assert_eq!(a, b);
        }
    }
}

#[test]
fn bootstrap_has_only_generated_imports_empty_resources_and_no_debug_or_signatures() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-application/hello.mc"
    )])
    .unwrap();
    let app = build_console_application(&unit, &options("main")).unwrap();
    assert_eq!(app.sections().len(), 8);
    assert!(app.unique_section(ciq_prg::SYMBOLS).is_err());
    assert!(app.unique_section(0xe1c0de12).is_err());
    let imports = app.unique_section(0xc1a557b1).unwrap().payload();
    assert_eq!(u16::from_be_bytes(imports[..2].try_into().unwrap()), 2);
    assert_eq!(word(&imports[2..6]), 0x800015);
    assert_eq!(word(&imports[6..10]), 0x80029c);
    assert_eq!(word(&imports[10..14]), 0x800011);
    assert_eq!(word(&imports[14..18]), 0x80019f);
    assert!(
        app.unique_section(ciq_prg::RESOURCES)
            .unwrap()
            .payload()
            .iter()
            .all(|b| *b == 0)
    );
    assert_eq!(
        app.unique_section(ciq_prg::EXCEPTIONS).unwrap().payload(),
        [0, 0]
    );
    assert_eq!(
        app.sections()
            .iter()
            .map(|s| 8 + s.payload().len())
            .sum::<usize>()
            % 32,
        0
    );
}

#[test]
fn sdk_member_names_share_ids_without_colliding_with_generated_app_class() {
    let unit = compile_sources(&["function main() { System.println(1); return A.println(); } function App() { return 3; } module A { function println() { return 2; } }"]).unwrap();
    let app = build_console_application(&unit, &options("main")).unwrap();
    let records = classes(&app);
    assert!(records[3].fields.iter().any(|f| f.symbol_id() == 0x800067));
    let globals = &records[1];
    assert_eq!(globals.fields.iter().filter(|f| f.kind == 7).count(), 1);
    assert_eq!(
        globals
            .fields
            .iter()
            .filter(|f| f.kind == 6 && f.symbol_id() != 0x800018)
            .count(),
        2
    );
    assert!(globals.fields.windows(2).all(|w| w[0].key < w[1].key));
}

#[test]
fn unsupported_entries_and_reserved_global_module_names_are_rejected() {
    for (source, entry) in [
        ("function f() { return 1; }", "missing"),
        ("module M { function f() { return 1; } }", "f"),
        ("function f(x) { return x; }", "f"),
        (
            "function main() { return 1; } function Toybox() { return 2; }",
            "main",
        ),
        (
            "function main() { return 1; } module globals { function f() { return 2; } }",
            "main",
        ),
        (
            "function main() { return 1; } module statics { function f() { return 2; } }",
            "main",
        ),
        (
            "function main() { return 1; } module Toybox { function f() { return 2; } }",
            "main",
        ),
    ] {
        let unit = compile_sources(&[source]).unwrap();
        assert!(
            build_console_application(&unit, &options(entry)).is_err(),
            "{source}"
        );
    }
}

#[test]
fn cli_generates_prg_from_source_and_ir_and_checks_application_options() {
    let root = std::env::temp_dir().join(format!("ciq-application-test-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("hello.mc");
    let ir = root.join("unit.ciqir");
    let direct = root.join("direct.prg");
    let resumed = root.join("resumed.prg");
    fs::write(
        &source,
        include_str!("../../../examples/compiler-application/hello.mc"),
    )
    .unwrap();
    let exe = env!("CARGO_BIN_EXE_ciq-compiler");
    let id = "41414141414141414141414141414141";
    let result = Command::new(exe)
        .args(["--unit", "--emit", "ir"])
        .arg(&source)
        .arg(&ir)
        .output()
        .unwrap();
    assert!(result.status.success(), "{:?}", result);
    for (from, input, out) in [("source", &source, &direct), ("ir", &ir, &resumed)] {
        let result = Command::new(exe)
            .args(["--unit", "--from", from, "--emit", "prg", "--app-id", id])
            .arg(input)
            .arg(out)
            .output()
            .unwrap();
        assert!(result.status.success(), "{:?}", result);
    }
    assert_eq!(fs::read(&direct).unwrap(), fs::read(&resumed).unwrap());
    assert_eq!(
        fs::read(&direct).unwrap(),
        build_console_application(
            &compile_sources(&[include_str!(
                "../../../examples/compiler-application/hello.mc"
            )])
            .unwrap(),
            &options("main")
        )
        .unwrap()
        .to_bytes()
    );
    for args in [
        vec!["--unit", "--emit", "prg"],
        vec!["--emit", "prg", "--app-id", id],
        vec!["--unit", "--emit", "prg", "--app-id", "éééééééééééééééé"],
        vec!["--unit", "--emit", "prg", "--app-id", "123"],
        vec![
            "--unit", "--emit", "prg", "--app-id", id, "--entry", "missing",
        ],
        vec!["--unit", "--emit", "prg", "--app-id", id, "--app-id", id],
        vec!["--unit", "--emit", "assembly", "--app-id", id],
        vec![
            "--unit",
            "--emit",
            "prg",
            "--app-id",
            id,
            "--symbols",
            "unused",
        ],
        vec![
            "--unit", "--from", "assembly", "--emit", "prg", "--app-id", id,
        ],
    ] {
        let out = root.join("invalid.prg");
        let result = Command::new(exe)
            .args(args)
            .arg(&source)
            .arg(&out)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(!out.exists());
    }
    let result = Command::new(exe)
        .args(["--unit", "--emit", "prg", "--app-id", id])
        .arg(&source)
        .arg(&direct)
        .output()
        .unwrap();
    assert!(!result.status.success());
    fs::remove_dir_all(root).unwrap();
}
