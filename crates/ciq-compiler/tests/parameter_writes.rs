use ciq_compiler::{Operation, compile, compile_sources, parse_ir, parse_unit_ir};

#[test]
fn parameter_writes_keep_declared_types_and_saved_stages() {
    for source in [
        "function f(n as Toybox.Lang.Number) as Toybox.Lang.Number { n=2; n+=3; n++; --n; return n; }",
        "function f(n as Toybox.Lang.String) as Toybox.Lang.String { n=\"other\"; return n; }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { n=null; n=1; if(n!=null){return n;} return 0; }",
        "function f(n as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { n=[]; n.add(1); return n.size(); }",
        "function f(n as Toybox.Lang.Object) as Toybox.Lang.Number { if(n instanceof Toybox.Lang.Number){var old=n; n=\"text\"; return old+1;} return 0; }",
        "function f(n as Toybox.Lang.Object) as Toybox.Lang.Number {var copy=n; if(copy instanceof Toybox.Lang.Number){n=\"text\"; return copy+1;} return 0; }",
        "function f(w as Toybox.Lang.WeakReference or Null) as Toybox.Lang.WeakReference or Null {w=null; return w;}",
        "function f(n as Toybox.Lang.Number) as Toybox.Lang.Number { var sum=0; for(n=0;n<3;n++){sum+=n;} return sum; }",
        "function f(n as Toybox.Lang.Number, b as Toybox.Lang.Boolean) as Toybox.Lang.Number { n+=b ? 2 : 3; return n; }",
    ] {
        let function = compile(source).unwrap();
        assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
        let assembly = function.lower();
        assert_eq!(
            assembly,
            ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
        );
    }
    let function =
        compile("function f(n as Toybox.Lang.Number) as Toybox.Lang.Number { n=2; return n; }")
            .unwrap();
    assert!(
        function
            .operations()
            .iter()
            .any(|op| matches!(op.operation, Operation::ParameterAssign { index: 0, .. }))
    );
    assert!(
        !function
            .operations()
            .iter()
            .any(|op| matches!(op.operation, Operation::LocalInitialize { .. }))
    );
    let instructions = ciq_mc::decode(&function.lower().encode().unwrap()).unwrap();
    assert!(
        instructions
            .iter()
            .any(|op| op.instruction.opcode() == ciq_mc::Opcode::Lputv
                && op.instruction.operands() == [1])
    );
}

#[test]
fn parameter_writes_reject_wrong_types_stale_guards_and_bad_indices() {
    for source in [
        "function f(n as Toybox.Lang.Number) as Toybox.Lang.Number { n=\"bad\"; return 0; }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { if(n!=null){n=null; return n+1;} return 0; }",
        "function f(n as Toybox.Lang.Object) as Toybox.Lang.Number { if(n instanceof Toybox.Lang.Number){n=\"bad\"; return n+1;} return 0; }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { if(n!=null){n++; return n;} return 0; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    for index in ["-1", "+0", "1", "999999999999999999999999999999999"] {
        let text = format!(
            "ciq-ir\n.function f parameters=1\n.integer %0 1\n.assign-parameter {index} %0\n.integer %1 0\n.return %1\n.end\n"
        );
        assert!(parse_ir(&text).is_err(), "{text}");
    }
    for annotation in [
        "Toybox.Lang.Number",
        "Toybox.Lang.String",
        "Toybox.Lang.Number or Null",
    ] {
        let error = compile(&format!(
            "function f() as Toybox.Lang.Number {{ var n as {annotation}=1; return 0; }}"
        ))
        .unwrap_err();
        assert!(error.message.contains("local variable types are inferred"));
    }
}

#[test]
fn source_and_interface_parameter_assignments_require_exact_witnesses() {
    let classes = "class Token {function initialize(){}} class Other {function initialize(){}} class Good {function initialize(){} function compareTo(other as Toybox.Lang.Object) as Toybox.Lang.Number {return 0;}}";
    for body in [
        "function f(t as Token or Null, other as Token) as Token or Null {t=other; t=null; return t;}",
        "function f(c as Toybox.Lang.Comparable) as Toybox.Lang.Number {c=new Good(); return 0;}",
        "function f(c as Toybox.Lang.Comparable, value as Good or Null) as Toybox.Lang.Number {if(value!=null){c=value;} return 0;}",
        "function f(c as Toybox.Lang.Comparable or Null, value as Good or Null) as Toybox.Lang.Number {if(value!=null){c=value;} else {c=null;} return 0;}",
    ] {
        let unit = compile_sources(&[classes, body]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
    for body in [
        "function f(t as Token) as Toybox.Lang.Number {t=new Other(); return 0;}",
        "function f(t as Token) as Toybox.Lang.Number {t=null; return 0;}",
        "function f(c as Toybox.Lang.Comparable) as Toybox.Lang.Number {c=new Other(); return 0;}",
        "function f(c as Toybox.Lang.Comparable, value as Good or Null) as Toybox.Lang.Number {c=value; return 0;}",
        "function f(c as Toybox.Lang.Comparable or Null, value as Good or Null) as Toybox.Lang.Number {c=value; return 0;}",
    ] {
        assert!(compile_sources(&[classes, body]).is_err(), "{body}");
    }
}
