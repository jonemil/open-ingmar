use ciq_compiler::{compile_sources, compile_sources_with_api, parse_unit_ir};
mod common;

#[test]
fn literal_reads_keep_module_activation_and_complete_saved_stages() {
    let sources = [
        include_str!("../../../examples/compiler-module-literals/app.mc"),
        include_str!("../../../examples/compiler-module-literals/later.mc"),
    ];
    let unit = compile_sources(&sources).unwrap();
    let text = unit.to_ir_text();
    for (owner, ty) in [
        ("Numbers", "integer"),
        ("Floats", "float"),
        ("Booleans", "boolean"),
        ("Chars", "char"),
        ("Symbols", "symbol"),
        ("Nulls", "null"),
    ] {
        assert!(text.contains(&format!(".shared-constant {owner} VALUE {ty}")));
        assert!(text.contains(&format!(" {owner} VALUE result={ty}")));
        assert!(
            unit.lower()
                .to_text()
                .contains(&format!("getmv @__ciq_module_{owner} @__ciq_field_VALUE"))
        );
    }
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x870326c7480145afbb0ed80764cb0721u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    common::pipeline(compile_sources(&[sources[1], sources[0]]).unwrap());
    let missing = text.replace(".shared-constant Numbers VALUE integer\n", "");
    assert_ne!(missing, text);
    assert!(parse_unit_ir(&missing).is_err());
}

#[test]
fn same_owner_literal_reads_use_the_current_receiver() {
    let unit = common::pipeline(
        compile_sources(&[
            "module M{const VALUE=7;const COPY=VALUE;function own(){return self.VALUE;}}",
        ])
        .unwrap(),
    );
    let assembly = unit.lower().to_text();
    assert!(assembly.contains("getselfv @__ciq_field_VALUE"));
    assert!(!assembly.contains("getmv @__ciq_module_M @__ciq_field_VALUE"));
}

#[test]
fn literal_activation_keeps_cycle_readonly_and_api_checks() {
    for source in [
        "module A{const VALUE=1;const OTHER=B.VALUE;}module B{const VALUE=2;const OTHER=A.VALUE;}function f(){return A.VALUE;}",
        "module M{const VALUE=1;const EFFECT=f();function f() as Toybox.Lang.Number{return VALUE;}}",
        "module M{const VALUE=1;}function f(){M.VALUE=2;}",
        "module M{const VALUE=1;}function f(){M.VALUE++;}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let low = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(1, 0, 0),
        ciq_compiler::api::ApiLevel(1, 0, 0),
    )
    .unwrap();
    assert!(compile_sources_with_api(&["module M{const UNUSED='A';}function f(){}"], low).is_err());

    let unit = compile_sources(&["module M{const VALUE=1;}class C{var value as Toybox.Lang.String;function initialize(){self.value=M.VALUE.toString();}}"]).unwrap();
    let text = unit.to_ir_text();
    let mut lines = Vec::new();
    for line in text.lines() {
        lines.push(line.to_owned());
        if line.starts_with(".assign-shared M VALUE ") {
            lines.push(".call-native-module-discard Toybox.System exit".into());
        }
    }
    let error = parse_unit_ir(&lines.join("\n")).unwrap_err();
    assert!(error.message.contains("shutdown"), "{error}");
}
