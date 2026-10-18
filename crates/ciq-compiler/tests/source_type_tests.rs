use ciq_compiler::{compile_sources, parse_ir, parse_unit_ir};

const CLASS: &str = "class C { function initialize() {} function value() { return 7; } } class Other { function initialize() {} }";

#[test]
fn source_targets_resolve_across_files_with_exact_provenance() {
    for body in [
        "function f(o as Toybox.Lang.Object) { if (o instanceof C) { return o.value(); } else { return 0; } }",
        "function f(o as Toybox.Lang.Object or Null) { if (!(o instanceof C)) { return 0; } return o.value(); }",
        "function f(o as Toybox.Lang.Object) { return (o as C).value(); }",
        "function f(o as Toybox.Lang.Object) as C { return o as C; }",
        "function f(o as Toybox.Lang.Object) { var x=o; while (x instanceof C) { System.println(x.value()); x=new Other(); } return 0; }",
        "function f(o as Toybox.Lang.Object) { return o instanceof C ? o.value() : 0; }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Boolean { return o instanceof C && o.value()>0; }",
        "module M { function take(c as C) { return c.value(); } function f(o as Toybox.Lang.Object) { if (o instanceof C) { return M.take(o); } else { return 0; } } }",
        "class Holder { var c as C; function initialize(o as Toybox.Lang.Object) { self.c=o as C; } function get() as C { return self.c; } }",
    ] {
        for sources in [[body, CLASS], [CLASS, body]] {
            let unit = compile_sources(&sources).unwrap_or_else(|e| panic!("{body}: {e}"));
            let text = unit.to_ir_text();
            assert!(text.contains("source(C@Lang.Object)"));
            assert_eq!(unit, parse_unit_ir(&text).unwrap());
            assert_eq!(
                unit.lower(),
                ciq_assembly::Program::parse(&unit.lower().to_text()).unwrap()
            );
        }
    }
    let source = "class V extends Toybox.WatchUi.View { function initialize() { View.initialize(); } function value() { return 1; } } function f(v as Toybox.WatchUi.View) { if (v instanceof V) { return v.value(); } else { return 0; } }";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn scoped_proofs_do_not_escape_to_aliases_writes_or_unchecked_paths() {
    for body in [
        "function f(o as Toybox.Lang.Object) { if (o instanceof C) { return 0; } else { return o.value(); } }",
        "function f(o as Toybox.Lang.Object) { var x=o; if (o instanceof C) { return x.value(); } else { return 0; } }",
        "function f(o as Toybox.Lang.Object) { var x=o; if (x instanceof C) { x=new Other(); return x.value(); } else { return 0; } }",
        "function f(o as Toybox.Lang.Object) { if (o instanceof C) { System.println(o.value()); } return o.value(); }",
        "function f(o as Toybox.Lang.Object) { var yes=o instanceof C; if (yes) { return o.value(); } else { return 0; } }",
        "function f(o as Toybox.Lang.Object) { var c=o as C; c=new Other(); return 0; }",
        "function f(o as Toybox.Lang.Object) { var c=o as C; var erased=c as Toybox.Lang.Object; return erased.value(); }",
        "function f(o as Toybox.Lang.Object) { return (o as Missing).value(); }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Boolean { return o instanceof Missing; }",
        "function f(o as Toybox.Lang.Object) { var c=o as C or Null; }",
        "function f(o as Toybox.Lang.Object) { var C=1; return (o as C).value(); }",
        "function f(o as Toybox.Lang.Object, C) as Toybox.Lang.Boolean { return o instanceof C; }",
    ] {
        assert!(compile_sources(&[body, CLASS]).is_err(), "{body}");
    }
}

#[test]
fn saved_targets_and_branch_proofs_are_revalidated() {
    for body in [
        "function f(o as Toybox.Lang.Object) { return (o as C).value(); }",
        "function f(o as Toybox.Lang.Object) { if (o instanceof C) { return o.value(); } else { return 0; } }",
    ] {
        let unit = compile_sources(&[body, CLASS]).unwrap();
        let text = unit.to_ir_text();
        for changed in [
            text.replace("source(C@Lang.Object)", "source(Missing@Lang.Object)"),
            text.replace("source(C@Lang.Object)", "source(C@WatchUi.View)"),
            text.replace("source(C@Lang.Object)", "source(Other@Lang.Object)"),
        ] {
            assert_ne!(text, changed);
            assert!(parse_unit_ir(&changed).is_err(), "{changed}");
        }
    }
    let text = "ciq-ir\n.function f parameters=1 parameter-types=object\n.block ^0\n.parameter %0 0\n.local $0 %0\n.read %1 $0\n.instanceof %2 %1 source(C@Lang.Object)\n.branch %2 ^1 ^2\n.block ^1\n.read %3 $0\n.call-object %4 %3 C value\n.return %4\n.block ^2\n.integer %5 0\n.return %5\n.end\n";
    parse_ir(text).unwrap(); // Declaration resolution remains a unit obligation.
    assert!(parse_ir(&text.replace(".branch %2 ^1 ^2", ".branch %2 ^1 ^1")).is_err());
    let stale = text.replace(".instanceof", ".integer %6 1\n.assign $0 %6\n.instanceof");
    assert!(parse_ir(&stale).is_err());
    parse_ir(&text.replace(".read %3 $0", ".take %3 $0")).unwrap();
}

#[test]
fn asserted_and_proven_classes_can_supply_structural_comparator_witnesses() {
    let class = "class Ordering { function initialize() {} function compare(a as Toybox.Lang.Object, b as Toybox.Lang.Object) as Toybox.Lang.Number { return 0; } }";
    for body in [
        "function f(o as Toybox.Lang.Object) { var a=[2,1]; a.sort(o as Ordering); }",
        "function f(o as Toybox.Lang.Object) { var a=[2,1]; if (o instanceof Ordering) { a.sort(o); } }",
    ] {
        let unit = compile_sources(&[body, class]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
}
