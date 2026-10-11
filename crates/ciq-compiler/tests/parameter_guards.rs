use ciq_compiler::{
    Function, Operation, ParameterType,
    api::{ApiContext, ApiLevel},
    compile, compile_sources, parse_ir, parse_unit_ir,
};

#[test]
fn direct_parameter_guards_preserve_source_and_ir_contracts() {
    for source in [
        "function f(x as Toybox.Lang.Number or Null) { if (x == null) { return 0; } return x + 1; }",
        "function f(x as Toybox.Lang.Number or Null) { if (null == x) { return 0; } return x.abs(); }",
        "function f(x as Toybox.Lang.String or Null) as Toybox.Lang.String { if (x == null) { return \"fallback\"; } return x; }",
        "function f(x as Toybox.Lang.String or Null) { if (x == null) { return 0; } if (x == null) { return 0; } return x.length(); }",
        "function f(x as Toybox.Lang.String or Null) { while (x == null) { return 0; } return x.length(); }",
        "function f(x as Toybox.Lang.String or Null, flag as Toybox.Lang.Boolean) { if (x == null) { return 0; } var local = 0; while (flag) { local = local + 1; break; } return x.length(); }",
        "function f(x as Toybox.Lang.String or Null, flag as Toybox.Lang.Boolean) { if (flag) { if (x == null) { return 0; } } else { if (x == null) { return 0; } } return x.length(); }",
        "function f(x as Toybox.Lang.String or Null) { if (x == null) { return 0; } var local = x; local = \"other\"; System.println(x); return x.length(); }",
    ] {
        let f = compile(source).unwrap();
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        f.validate_api(ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap())
            .unwrap();
    }
    let unit = compile_sources(&[
        "module M { function f(x as Toybox.Lang.String or Null) { if (x == null) { return 0; } return M.length(x); } function length(s as Toybox.Lang.String) { return s.length(); } }",
    ]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn guards_do_not_prove_other_parameters_aliases_or_unguarded_paths() {
    for source in [
        "function f(x as Toybox.Lang.String or Null) { return x.length(); }",
        "function f(x as Toybox.Lang.String or Null) { if (x == null) { return x.length(); } return 0; }",
        "function f(x as Toybox.Lang.String or Null, y as Toybox.Lang.String or Null) { if (x == null) { return 0; } return y.length(); }",
        "function f(x as Toybox.Lang.String or Null) { var local = x; if (local == null) { return 0; } return x.length(); }",
        "function f(x as Toybox.Lang.String or Null) { var local = x; if (x == null) { return 0; } return local.length(); }",
        "function f(x as Toybox.Lang.String or Null) { if (x == null) { System.println(0); } else { System.println(x); } return x.length(); }",
        "function f(x as Toybox.Lang.String or Null) { var flag = x == null; if (flag) { return 0; } return x.length(); }",
        "function f(x as Toybox.Lang.String or Null) { var nil = null; if (x == nil) { return 0; } return x.length(); }",
        "function f(x as Toybox.Lang.String or Null) { while (x == null) { break; } return x.length(); }",
        "function f(x as Toybox.Lang.String or Null) { while (true) { System.println(x); if (x == null) { return 0; } } return 0; }",
        "function f(x as Toybox.Lang.String or Null) { if (x == null) { return 0; } x = null; return x.length(); }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn edited_parameter_guard_ir_cannot_trust_stale_proofs() {
    let f = compile("function f(x as Toybox.Lang.String or Null, y as Toybox.Lang.String or Null) { if (x == null) { return 0; } return x.length(); }").unwrap();
    let types = vec![ParameterType::NullableString; 2];
    let mut swapped = f.operations().to_vec();
    for op in &mut swapped {
        if let Operation::Branch {
            on_true, on_false, ..
        } = &mut op.operation
        {
            std::mem::swap(on_true, on_false);
        }
    }
    assert!(Function::with_parameter_types("unsafe".into(), types.clone(), swapped).is_err());
    let mut different = f.operations().to_vec();
    let op = different
        .iter_mut()
        .find(|op| matches!(op.operation, Operation::ParameterRead { .. }))
        .unwrap();
    let Operation::ParameterRead { index, .. } = &mut op.operation else {
        unreachable!()
    };
    *index = 1;
    assert!(Function::with_parameter_types("unsafe".into(), types, different).is_err());
    let same_target = "ciq-ir\n.function f parameters=1 parameter-types=nullable-string\n.block ^0\n.parameter %0 0\n.null %1\n.equal %2 %0 %1\n.branch %2 ^1 ^1\n.block ^1\n.parameter %3 0\n.call-native %4 %3 length\n.return %4\n.end";
    assert!(parse_ir(same_target).is_err());
}
