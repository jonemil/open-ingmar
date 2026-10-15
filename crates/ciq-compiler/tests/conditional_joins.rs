use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    parse_ir, parse_ir_with_api,
};

const JOIN: &str = "ciq-ir\n.function f\n.block ^0\n.boolean %0 true\n.branch %0 ^1 ^2\n.block ^1\n.integer %1 11\n.jump-value ^3 %1\n.block ^2\n.integer %2 22\n.jump-value ^3 %2\n.block ^3\n.block-value %3 integer\n.return %3\n.end\n";

#[test]
fn typed_edges_roundtrip_and_lower_without_placeholder_or_frame_storage() {
    let f = parse_ir(JOIN).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let p = f.lower();
    assert_eq!(p, ciq_assembly::Program::parse(&p.to_text()).unwrap());
    let text = p.to_text();
    assert!(!text.contains("lputv"));
    assert!(!text.contains("argcincsp"));
    assert!(!text.contains("ipushz"));
    p.encode().unwrap();
    for (value, tag) in [(".char", "char"), (".boolean", "boolean")] {
        let input = JOIN
            .replace(
                ".integer %1 11",
                &format!(
                    "{value} %1 {}",
                    if tag == "char" { "0x00000061" } else { "true" }
                ),
            )
            .replace(
                ".integer %2 22",
                &format!(
                    "{value} %2 {}",
                    if tag == "char" { "0x00000062" } else { "false" }
                ),
            )
            .replace("%3 integer", &format!("%3 {tag}"));
        let f = parse_ir(&input).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        if tag == "char" {
            let old = ApiContext::new(ApiLevel(1, 2, 9), ApiLevel(1, 0, 0)).unwrap();
            assert!(parse_ir_with_api(&input, old).is_err());
        }
    }
    let nullable = JOIN
        .replace(".function f", ".function f result=nullable-integer")
        .replace(".integer %2 22", ".null %2")
        .replace("%3 integer", "%3 nullable-integer");
    assert_eq!(
        parse_ir(&nullable).unwrap(),
        parse_ir(&parse_ir(&nullable).unwrap().to_ir_text()).unwrap()
    );
}

#[test]
fn malformed_edge_shapes_types_uses_and_stack_order_are_rejected() {
    for bad in [
        JOIN.replace(".jump-value ^3 %1", ".jump ^3"),
        JOIN.replace(".jump-value ^3 %1", ".jump-value ^2 %1"),
        JOIN.replace(".branch %0 ^1 ^2", ".branch %0 ^3 ^2"),
        JOIN.replace(".integer %1 11", ".char %1 97"),
        JOIN.replace(".block-value %3 integer", ".block-value %3 void"),
        JOIN.replace(".block ^0", ".block ^0\n.block-value %9 integer"),
        JOIN.replace(
            ".block-value %3 integer",
            ".integer %9 1\n.block-value %3 integer",
        ),
        JOIN.replace(".jump-value ^3 %1", ".jump-value ^3 %2"),
        JOIN.replace(".return %3", ".integer %4 1\n.add %5 %4 %3\n.return %5"),
        JOIN.replace(".return %3", ".integer %4 1\n.add %5 %3 %4\n.return %3"),
        JOIN.replace(".jump-value ^3 %1", ".jump-value ^3 %1 %1"),
        JOIN.replace(".return %3", ".println %3\n.integer %4 1\n.return %4"),
        JOIN.replace(".return %3", ".call %4 M f %3\n.return %4"),
    ] {
        assert!(parse_ir(&bad).is_err(), "accepted {bad}");
    }
    let good = JOIN.replace(".return %3", ".integer %4 1\n.add %5 %3 %4\n.return %5");
    parse_ir(&good).unwrap().lower().encode().unwrap();
}

#[test]
fn source_conditionals_preserve_pipeline_types_and_reject_unsupported_joins() {
    use ciq_compiler::{compile, compile_sources, parse_unit_ir};
    for source in [
        "function f(b as Toybox.Lang.Boolean) { return b ? 1 : 2; }",
        "function f() as Toybox.Lang.Char { return true ? 'a' : 'b'; }",
        "function f() as Toybox.Lang.Boolean { return true ? false : true ? true : true; }",
        "function f() as Toybox.Lang.Number or Null { return false ? 1 : null; }",
        "function f() as Toybox.Lang.String or Null { return false ? null : \"x\"; }",
        "function f() { var n = \"7\".toNumber(); return n != null ? n + 1 : 0; }",
        "function f() { var a = [1]; return (true ? a : [2])[0]; }",
        "function f() { return 1 + (false ? 2 : 3); }",
        "function f() as Toybox.Lang.Boolean { return !(true ? false : true); }",
        "function f() { var a = [true ? 1 : 2, false ? 3 : 4]; return a[0]; }",
    ] {
        let f = compile(source).unwrap_or_else(|e| panic!("{source}: {e:?}"));
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        let p = f.lower();
        assert_eq!(p, ciq_assembly::Program::parse(&p.to_text()).unwrap());
    }
    for source in [
        "function f() { return 1 ? 1 : 2; }",
        "function f() { return true ? 'a' : 1; }",
        "function f() { return true ? 1 : \"x\"; }",
        "function f() { return true ? [1] : ['a']; }",
        "function f() { return true ? [1] : null; }",
        "function f() { return true ? [] : [1]; }",
        "function f() { return true ? 1 : unknown; }",
        "function f() { return true ? 1; }",
        "function f() { return true ? : 2; }",
        "function f() { return true ? 1 :; }",
        "function f() { var n = \"x\".toNumber(); return n == null ? n + 1 : 0; }",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-conditionals/app.mc"
    )])
    .unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let app = ciq_compiler::application::ConsoleApplication {
        id: [0x39; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        ciq_compiler::application::build_console_application(&unit, &app).unwrap(),
        ciq_compiler::application::build_console_application(
            &parse_unit_ir(&unit.to_ir_text()).unwrap(),
            &app
        )
        .unwrap()
    );
}

#[test]
fn conditional_depth_and_frame_boundaries_remain_enforced() {
    use ciq_compiler::compile;
    let parameters = (0..253)
        .map(|i| format!("p{i}"))
        .chain(std::iter::once("b as Toybox.Lang.Boolean".into()))
        .collect::<Vec<_>>()
        .join(",");
    let f = compile(&format!("function f({parameters}) {{ return b ? 1 : 2; }}")).unwrap();
    assert!(!f.lower().to_text().contains("argcincsp"));
    compile(&format!(
        "function f({parameters}) {{ System.println(true && false); return 1; }}"
    ))
    .unwrap();
    assert!(
        compile(&format!(
            "function f({parameters}) {{ return 1 + (b ? 1 : 2); }}"
        ))
        .is_err()
    );
    for (depth, ok) in [(64, true), (65, false)] {
        let expression = format!("{}1{}", "true ? ".repeat(depth), " : 2".repeat(depth));
        assert_eq!(
            compile(&format!("function f() {{ return {expression}; }}")).is_ok(),
            ok
        );
    }
}
