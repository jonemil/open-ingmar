use ciq_compiler::{compile_function, compile_sources, parse_function_ir, parse_unit_ir};

#[test]
fn positive_edges_refine_reads_in_branches_loops_and_logical_expressions() {
    for source in [
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { if (o instanceof Lang.Number) { return o+1; } else { return 0; } }",
        "function f(o as Toybox.Lang.Object or Null) as Toybox.Lang.Number { if (o instanceof Lang.String) { return o.length(); } else { return 0; } }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { if (!(o instanceof Lang.Number)) { return 0; } return o+1; }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Boolean { return o instanceof Lang.Number && o>0; }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Boolean { return !(o instanceof Lang.String) || o.length()>0; }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { return o instanceof Lang.String ? o.length() : 0; }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { var x=o; if (x instanceof Lang.Number) { var y=x; return y+1; } else { return 0; } }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { var x=o; while (x instanceof Lang.Number) { System.println(x+1); x=\"done\"; } return 0; }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { if (o instanceof Lang.String) { var x=0; while (x<2) { System.println(o.length()); x=x+1; } return o.length(); } else { return 0; } }",
        "function f(o as Toybox.Lang.Object, flag as Toybox.Lang.Boolean) as Toybox.Lang.Number { if (flag) { if (!(o instanceof Lang.String)) { return 0; } } else { if (!(o instanceof Lang.String)) { return 0; } } return o.length(); }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Long { if (o instanceof Lang.Long) { return -o; } else { return 0l; } }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Float { if (o instanceof Lang.Float) { return o+1.0; } else { return 0.0; } }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Double { if (o instanceof Lang.Double) { return o*2.0d; } else { return 0.0d; } }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { if (o instanceof Lang.Dictionary) { return o.size(); } else { return 0; } }",
        "function f(o as Toybox.Lang.Object or Null) as Toybox.Lang.Boolean { return o instanceof Lang.Object && o has :toString; }",
    ] {
        let function = compile_function(source).unwrap_or_else(|error| panic!("{source}: {error}"));
        let text = function.to_ir_text();
        assert_eq!(function, parse_function_ir(&text).unwrap());
        assert_eq!(function.lower(), parse_function_ir(&text).unwrap().lower());
    }
    let source = "module M { function length(s as Toybox.Lang.String) as Toybox.Lang.Number { return s.length(); } function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { if (o instanceof Lang.String) { return M.length(o); } else { return 0; } } }";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn false_edges_aliases_writes_joins_and_stored_booleans_do_not_forge_types() {
    for source in [
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { return o+1; }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { if (o instanceof Lang.Number) { return 0; } else { return o+1; } }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { var x=o; if (o instanceof Lang.Number) { return x+1; } else { return 0; } }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { var x=o; if (x instanceof Lang.Number) { return o+1; } else { return 0; } }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { var yes=o instanceof Lang.Number; if (yes) { return o+1; } else { return 0; } }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { if (o instanceof Lang.Number) { System.println(o+1); } return o+1; }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { var x=o; if (x instanceof Lang.Number) { x=\"changed\"; return x+1; } else { return 0; } }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { var x=o; while (x instanceof Lang.Number) { x=\"changed\"; break; } return x+1; }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { if (o has :length) { return o.length(); } else { return 0; } }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Number { if (o instanceof Lang.Array) { return o[0]; } else { return 0; } }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
}

#[test]
fn edited_ir_recomputes_branch_and_storage_proofs() {
    let function = compile_function("function f(o as Toybox.Lang.Object, other as Toybox.Lang.Object) as Toybox.Lang.Number { if (o instanceof Lang.String) { return o.length(); } else { return 0; } }").unwrap();
    let text = function.to_ir_text();
    let changed = text.replace("Toybox.Lang.String", "Toybox.Lang.Number");
    assert_ne!(text, changed);
    assert!(parse_function_ir(&changed).is_err());
    let mut operations = function.operations().to_vec();
    for located in &mut operations {
        if let ciq_compiler::Operation::Branch {
            on_true, on_false, ..
        } = &mut located.operation
        {
            std::mem::swap(on_true, on_false);
        }
    }
    assert!(
        ciq_compiler::Function::with_parameter_types(
            "f".into(),
            function.parameter_types().to_vec(),
            operations
        )
        .is_err()
    );
}

#[test]
fn saved_ir_rejects_same_target_edges_and_reads_stale_before_assignment() {
    let text = "ciq-ir\n.function f parameters=1 parameter-types=object\n.block ^0\n.parameter %0 0\n.local $0 %0\n.read %1 $0\n.instanceof %2 %1 Toybox.Lang.String\n.branch %2 ^1 ^2\n.block ^1\n.read %3 $0\n.call-native %4 %3 length\n.return %4\n.block ^2\n.integer %5 0\n.return %5\n.end\n";
    parse_function_ir(text).unwrap();
    let changed = text.replace(".instanceof", ".integer %6 1\n.assign $0 %6\n.instanceof");
    assert!(parse_function_ir(&changed).is_err());
    let changed = text.replace(".branch %2 ^1 ^2", ".branch %2 ^1 ^1");
    assert!(parse_function_ir(&changed).is_err());
}
