use ciq_compiler::{
    Operation, ReturnType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_function_with_api, compile_sources, parse_function_ir,
    parse_function_ir_with_api, parse_unit_ir,
};

#[test]
fn complete_byte_array_program_preserves_native_text_and_application_boundaries() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-byte-arrays/app.mc"
    )])
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [0x71; 16],
        entry: "MinimalConsoleApp.onStart".into(),
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

#[test]
fn byte_array_allocation_uses_newba_and_keeps_nonnull_storage_identity() {
    let source = "function f(n as Toybox.Lang.Number) as Toybox.Lang.ByteArray { return new[n]b; }";
    let f = compile_function(source).unwrap();
    assert_eq!(f.return_type(), ReturnType::ByteArray);
    assert!(
        f.operations()
            .iter()
            .any(|op| matches!(op.operation, Operation::ByteArrayAllocate { .. }))
    );
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    assert_eq!(f.lower().encode().unwrap(), [0x35, 2, 0x12, 1, 0x36, 0x16]);
    for source in [
        "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.ByteArray { return flag ? new[1]b : new[2]b; }",
        "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.ByteArray { return new[flag ? 1 : 2]b; }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.ByteArray { if (n != null) { return new[n]b; } return new[0]b; }",
        "function f() as Toybox.Lang.Number { var a = new[1]b; var shared = a; a = new[2]b; shared[0] = 'A'; return shared[0]; }",
        "function f() as Toybox.Lang.ByteArray { return new[-1]b; }",
    ] {
        let f = compile_function(source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        f.lower().encode().unwrap();
    }
}

#[test]
fn invalid_byte_lengths_writes_methods_and_storage_forms_are_rejected() {
    for source in [
        "function f() as Toybox.Lang.Number { return new[null]b; }",
        "function f() as Toybox.Lang.Number { return new[true]b; }",
        "function f() as Toybox.Lang.Number { return new[1l]b; }",
        "function f() as Toybox.Lang.Number { return new[1.0]b; }",
        "function f() as Void { var a = new[1]b; a[0] = null; }",
        "function f() as Void { var a = new[1]b; a[0] = true; }",
        "function f() as Void { var a = new[1]b; a[0] = 1l; }",
        "function f() as Toybox.Lang.Number { var a = new[1]b; return a['a']; }",
        "function f() as Void { var a = new[1]b; a.add(null); }",
        "function f() as Void { var a = new[1]b; a.addAll([true]); }",
        "function f() as Void { var a = new[1]b; a.addAll([\"x\"]); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number or Null>) as Void { var b = new[0]b; b.addAll(a); }",
        "function f() as Void { var a = new[1]b; a.remove(); }",
        "function f() as Void { var a = new[1]b; a.slice(0); }",
        "function f() as Void { var a = new[1]b; a.sort(null); }",
        "function f() as Toybox.Lang.ByteArray { return [1]; }",
        "function f() as Toybox.Lang.ByteArray { return null; }",
        "function f() as Toybox.Lang.Number { return [1, 2]b; }",
        "function f() as Toybox.Lang.Number { var a = new[0]b; return true ? a : null; }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    // Byte bounds remain runtime checks, even for literals known at compile time.
    for source in [
        "function f() as Toybox.Lang.Number { var a = new[1]b; a[0] = 256; return a[0]; }",
        "function f() as Toybox.Lang.Number { var a = new[0]b; a.add(-129); return a.size(); }",
    ] {
        assert!(compile_function(source).is_ok());
    }
}

#[test]
fn source_and_saved_ir_recheck_byte_array_api_and_length_operands() {
    let source = "function f(n as Toybox.Lang.Number) as Toybox.Lang.ByteArray { return new[n]b; }";
    let current = ApiContext::new(ApiLevel(3, 0, 0), ApiLevel(3, 0, 0)).unwrap();
    let old = ApiContext::new(ApiLevel(2, 4, 0), ApiLevel(2, 4, 0)).unwrap();
    let f = compile_function_with_api(source, current).unwrap();
    let text = f.to_ir_text();
    assert_eq!(f, parse_function_ir_with_api(&text, current).unwrap());
    assert!(compile_function_with_api(source, old).is_err());
    assert!(parse_function_ir_with_api(&text, old).is_err());
    let text = compile_function("function f() as Toybox.Lang.ByteArray { return new[2]b; }")
        .unwrap()
        .to_ir_text();
    for bad in [
        text.replace(".integer %0 2", ".boolean %0 true"),
        text.replace(".byte-array-new %1 %0", ".byte-array-new %1 %999"),
        text.replace(".byte-array-new %1 %0", ".byte-array-new %1 %0 %0"),
    ] {
        assert_ne!(bad, text);
        assert!(parse_function_ir(&bad).is_err());
    }
    let text = compile_function(
        "function f(b as Toybox.Lang.ByteArray) as Toybox.Lang.Number { b[0] = 7; return b[0]; }",
    )
    .unwrap()
    .to_ir_text();
    // Locate by payload instead of relying on source-emission IDs.
    let bad = text
        .lines()
        .map(|line| {
            if line.starts_with(".integer ") && line.split_whitespace().nth(2) == Some("7") {
                line.replacen(".integer", ".boolean", 1)
                    .replacen(" 7", " true", 1)
            } else {
                line.into()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(bad, text);
    assert!(parse_function_ir(&bad).is_err());
}
