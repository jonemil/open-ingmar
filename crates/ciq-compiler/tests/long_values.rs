use ciq_compiler::{
    ReturnType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, compile_with_api, parse_ir, parse_unit_ir,
};

#[test]
fn long_literal_bits_survive_ir_assembly_and_vm_codec() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    for (literal, value) in [
        ("0l", 0i64),
        ("1L", 1),
        ("-1l", -1),
        ("4294967296l", 4294967296),
        ("72623859790382856l", 0x0102030405060708),
        ("9223372036854775807l", i64::MAX),
        ("-9223372036854775808L", i64::MIN),
        ("0xffffffffL", 4294967295),
        ("077L", 63),
        ("0xffffffffffffffffL", -1),
        ("01777777777777777777777l", -1),
        ("0x8000000000000000L", i64::MIN),
        ("-0xffffffffffffffffL", 1),
        ("-0x8000000000000000L", i64::MIN),
    ] {
        let f = compile_with_api(
            &format!("function f() as Toybox.Lang.Long {{ return {literal}; }}"),
            initial,
        )
        .unwrap();
        assert_eq!(f.return_type(), ReturnType::Long);
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        let assembly = f.lower();
        assert_eq!(
            assembly,
            ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
        );
        let code = assembly.encode().unwrap();
        let decoded = ciq_mc::decode(&code).unwrap();
        let push = decoded
            .iter()
            .find(|i| {
                matches!(
                    i.instruction.opcode(),
                    ciq_mc::Opcode::Lpush | ciq_mc::Opcode::Lpushz
                )
            })
            .unwrap();
        if value == 0 {
            assert_eq!(push.instruction.opcode(), ciq_mc::Opcode::Lpushz);
        } else {
            assert_eq!(push.instruction.operands(), value.to_be_bytes());
        }
        let text = f.to_ir_text();
        for bad in [
            "9223372036854775808",
            "-9223372036854775809",
            "0x10",
            "+1",
            "1l",
        ] {
            assert!(
                parse_ir(&text.replace(&format!(".long %0 {value}"), &format!(".long %0 {bad}")))
                    .is_err()
            );
        }
    }
}

#[test]
fn long_calls_and_complete_app_preserve_all_saved_stages() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-long-values/app.mc"
    )])
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let app = ConsoleApplication {
        id: [71; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        build_console_application(&unit, &app).unwrap(),
        build_console_application(&resumed, &app).unwrap()
    );
    let text = unit.to_ir_text();
    let edited = text.replace("pass result=long", "pass result=integer");
    assert_ne!(text, edited);
    assert!(parse_unit_ir(&edited).is_err());
    let source = "module Wide { function pass(value as Toybox.Lang.Long) as Toybox.Lang.Long { return value; } } function f() as Toybox.Lang.Long { return Wide.pass(1); }";
    assert!(compile_sources(&[source]).is_err());
}

#[test]
fn long_subset_rejects_narrowing_unsupported_operators_and_containers() {
    for source in [
        "function f() as Toybox.Lang.Long { return 1; }",
        "function f() { return 1l; }",
        "function f() as Toybox.Lang.Long { return 9223372036854775808l; }",
        "function f() as Toybox.Lang.Long { return -9223372036854775809l; }",
        "function f() as Toybox.Lang.Long { return 0x10000000000000000L; }",
        "function f() as Toybox.Lang.Long { return 02000000000000000000000L; }",
        "function f() as Toybox.Lang.Long { return 08L; }",
        "function f() as Toybox.Lang.Long { return 2147483648; }",
        "function f() as Toybox.Lang.Long { return 1l & true; }",
        "function f() as Toybox.Lang.Boolean { return 1l == true; }",
        "function f() as Toybox.Lang.Long { return ~'A'; }",
        "function f() { var value = 1l; value = 1; }",
        "function f() { var value = 1; value = 1l; }",
        "function f(x as Toybox.Lang.Long or Null) as Toybox.Lang.Long { return x.abs(); }",
        "function f() { return 1l.toNumber(1); }",
        "function f() { var values = [1l, 1]; }",
        "function f() { var values = {1 => 1l, 2 => 1}; }",
        "function f(x as Toybox.Lang.Array<Toybox.Lang.Long>) { x.add(1); }",
        "function f(x as Toybox.Lang.Dictionary<Toybox.Lang.Number, Toybox.Lang.Long>) { x.put(1, 1); }",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
}
