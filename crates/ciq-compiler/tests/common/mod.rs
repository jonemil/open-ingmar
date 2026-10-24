use ciq_compiler::{
    CompilationUnit,
    application::{ConsoleApplication, build_console_application},
    parse_unit_ir,
};

/// Check saved IR and assembly text preserve native lowering. Return the
/// reloaded unit so callers can exercise later stages from saved IR too.
pub fn pipeline(unit: CompilationUnit) -> CompilationUnit {
    let resumed = parse_unit_ir(&unit.to_ir_text()).expect("reload semantic IR");
    assert_eq!(unit, resumed, "saved IR changes the semantic unit");
    let assembly = unit.lower();
    assert_eq!(assembly, resumed.lower(), "saved IR changes lowering");
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).expect("reload assembly"),
        "assembly text changes the lowered program",
    );
    resumed
}

/// Check complete unsigned PRG generation from native and saved IR, including
/// framing, lossless CODE decoding, and a complete DATA walk. SDK/CLI
/// reconstruction, signing, and runtime behavior remain separate checks.
pub fn application(unit: &CompilationUnit, options: &ConsoleApplication) -> ciq_prg::Prg {
    let resumed = pipeline(unit.clone());
    let direct = build_console_application(unit, options).expect("build native application");
    let reloaded =
        build_console_application(&resumed, options).expect("build saved-IR application");
    assert_eq!(
        direct.to_bytes(),
        reloaded.to_bytes(),
        "saved IR changes PRG output"
    );
    assert_eq!(
        direct,
        ciq_prg::Prg::parse(&direct.to_bytes()).expect("parse generated PRG"),
        "PRG serialization changes sections",
    );
    let code = direct
        .unique_section(ciq_prg::CODE)
        .expect("unique CODE")
        .payload();
    let decoded = ciq_mc::decode(code).expect("decode all generated CODE");
    let mut encoded = Vec::new();
    for located in decoded {
        located.instruction.encode_into(&mut encoded);
    }
    assert_eq!(code, encoded, "CODE decoding changes instruction bytes");
    let data = direct
        .unique_section(ciq_prg::DATA)
        .expect("unique DATA")
        .payload();
    ciq_prg::data::classes::records(data).expect("walk all generated DATA records");
    direct
}
