#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_sources, parse_function_ir, parse_unit_ir,
};

#[test]
fn nullable_native_calls_preserve_complete_app_and_saved_ir() {
    let unit = compile_sources(&[
        include_str!("../../../examples/compiler-nullable-numbers/app.mc"),
        include_str!("../../../examples/compiler-nullable-numbers/numbers.mc"),
    ])
    .unwrap();
    let context = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(1, 0, 0),
        ciq_compiler::api::ApiLevel(1, 0, 0),
    )
    .unwrap();
    unit.validate_api(context).unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [21; 16],
        entry: "start".into(),
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
    assert!(assembly.to_text().contains("getsv @__ciq_native_find"));
    assert!(
        assembly
            .to_text()
            .contains("getsv @__ciq_native_toNumber\ninvokemz")
    );
    let symbols = ciq_assembly::Program::parse(".symbol globals \"globals\" id=0x008002e6\n.symbol find \"find\" id=0x008000bc\n.symbol number \"toNumber\" id=0x008000a7\n.symbol system \"Toybox_System\" id=0x00800005\n.symbol println \"println\" id=0x00800067\n.symbol format \"format\" id=0x0080009c\n.symbol exit \"exit\" id=0x0080006d\n.symbol abs \"abs\" id=0x008000a6\n.symbol numbers \"globals/Numbers\" id=0x00000001\n.symbol show \"showFind\" id=0x00000002\n.symbol parse \"parseOrZero\" id=0x00000003").unwrap();
    let bindings = ciq_compiler::symbol_bindings(&assembly, &symbols).unwrap();
    assert_eq!(bindings["__ciq_native_find"], 0x008000bc);
    assert_eq!(bindings["__ciq_native_toNumber"], 0x008000a7);
}

#[test]
fn null_guards_narrow_only_the_checked_local_on_the_non_null_path() {
    for source in [
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); if (n == null) { return 0; } else { return n + 1; } }",
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); if (null == n) { return 0; } return n.abs(); }",
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); if (n == null) { return 0; } return M.consume(n); }",
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); while (n == null) { n = \"7\".toNumber(); } return n; }",
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); if (n == null) { return 0; } if (n == null) { return 0; } return n; }",
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); if (n == null) { return 0; } while (false) { System.println(n); } return n; }",
    ] {
        let function = compile_function(source).unwrap();
        assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
    }
    for source in [
        "function f() as Toybox.Lang.Number { return \"41\".toNumber(); }",
        "function f() as Void { System.println(\"41\".toNumber()); }",
        "function f() as Toybox.Lang.Number { return \"41\".toNumber() + 1; }",
        "function f() as Toybox.Lang.Number { return \"41\".toNumber().abs(); }",
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); if (n == null) { return n + 1; } else { return 0; } }",
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); if (n == null) { System.println(0); } else { System.println(n); } return n; }",
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); if (n == null) { return 0; } n = null; return n; }",
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); if (n == null) { return 0; } n = null; return n+1; }",
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); if (n == null) { return 0; } n = \"bad\".toNumber(); return n; }",
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); var other = \"7\".toNumber(); if (n == null) { return 0; } return other; }",
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); while (n == null) { break; } return n; }",
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); var flag = n == null; if (flag) { return 0; } return n; }",
        "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Number { var n = \"41\".toNumber(); if (n == null) { return 0; } while (flag) { n = null; } return n; }",
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); if (n == null) { return 0; } n = true; return n+1; }",
    ] {
        assert!(checking::compile_function(source).is_err(), "{source}");
    }
}

#[test]
fn ir_edits_cannot_bypass_non_null_proofs() {
    let function = compile_function(
        "function f() as Toybox.Lang.Number { var n = \"41\".toNumber(); if (n == null) { return 0; } return n + 1; }",
    )
    .unwrap();
    let mut operations = function.operations().to_vec();
    let branch = operations
        .iter_mut()
        .find(|op| matches!(op.operation, ciq_compiler::Operation::Branch { .. }))
        .unwrap();
    let ciq_compiler::Operation::Branch {
        on_true, on_false, ..
    } = &mut branch.operation
    else {
        unreachable!()
    };
    std::mem::swap(on_true, on_false);
    assert!(ciq_compiler::Function::new("bad".into(), operations).is_err());

    let same_target = "ciq-ir\n.function f\n.block ^0\n.string %0 \"41\"\n.call-native %1 %0 toNumber\n.local $0 %1\n.read %2 $0\n.null %3\n.equal %4 %2 %3\n.branch %4 ^1 ^1\n.block ^1\n.read %5 $0\n.return %5\n.end";
    assert!(parse_function_ir(same_target).is_err());
    let reassigned_after_read = "ciq-ir\n.function f\n.block ^0\n.string %0 \"41\"\n.call-native %1 %0 toNumber\n.local $0 %1\n.read %2 $0\n.null %3\n.equal %4 %2 %3\n.null %5\n.assign $0 %5\n.branch %4 ^1 ^2\n.block ^1\n.integer %6 0\n.return %6\n.block ^2\n.read %7 $0\n.return %7\n.end";
    assert!(parse_function_ir(reassigned_after_read).is_err());
}

#[test]
fn nullable_methods_reject_bad_arguments_and_other_receivers() {
    for source in [
        "function f() as Void { var n = \"text\".find(); }",
        "function f() as Void { var n = \"text\".find(1); }",
        "function f() as Void { var n = \"text\".find(null); }",
        "function f() as Void { var n = \"text\".toNumber(1); }",
        "function f() as Void { var n = true.toNumber(); }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
}
