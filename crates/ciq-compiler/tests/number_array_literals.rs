use ciq_compiler::{
    ArrayElement, Operation, ReturnType,
    application::{ConsoleApplication, build_console_application},
    compile, compile_sources, parse_ir, parse_unit_ir,
};

#[test]
fn inferred_literals_keep_concrete_contracts_through_both_text_boundaries() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-number-array-literals/app.mc"
    )])
    .unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [45; 16],
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
    for source in [
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number> { return [1, 2]; }",
        "function f(n as Toybox.Lang.Number) as Toybox.Lang.Array<Toybox.Lang.Number> { return [n, -n]; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number> { return [(true || false).hashCode(), 1]; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number> { var n = 1; n = 2; return [n, n + 3]; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Array<Toybox.Lang.Number> { return [a[0], 2]; }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Array<Toybox.Lang.Number> { if (n == null) { return [0]; } return [n, n + 1]; }",
        "function f(s as Toybox.Lang.String or Null) as Toybox.Lang.Array<Toybox.Lang.Number> { if (s == null) { return [0]; } return [s.length(), 2]; }",
    ] {
        let f = compile(source).unwrap();
        assert_eq!(f.return_type(), ReturnType::Array(ArrayElement::Number));
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    }
    let f = compile("function f() as Toybox.Lang.Array<Toybox.Lang.Number> { return [1, 2]; }")
        .unwrap();
    let text = f.to_ir_text();
    assert!(text.contains(".array-constant %0 number"));
    assert_eq!(
        parse_ir(&text.replace(".array-constant %0 number", ".array-constant %0 char"))
            .unwrap()
            .return_type(),
        ReturnType::Array(ArrayElement::Char),
    );
}

#[test]
fn candidate_types_do_not_authorize_invalid_elements_or_null_uses() {
    for source in [
        "function f() as Void { var a = [1, 'a']; }",
        "function f() as Void { var a = ['a', 1]; }",
        "function f() as Void { var a = [true, 1]; }",
        "function f() as Void { var a = [\"a\", 1]; }",
        "function f() as Void { var a = [null]; }",
        "function f() as Void { var a = [[1]]; }",
        "function f() as Void { var a = [1, null]; }",
        "function f(n as Toybox.Lang.Number or Null) as Void { var a = [n, 1]; }",
        "function f(n as Toybox.Lang.Number or Null) as Void { var a = [n + 1]; }",
        "function f(s as Toybox.Lang.String or Null) as Void { var a = [s.length()]; }",
        "function f() as Void { var n = \"x\".toNumber(); var a = [n]; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Char> { return [1]; }",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let f = compile("function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Array<Toybox.Lang.Number> { if (n == null) { return [0]; } return [n]; }").unwrap();
    let text = f.to_ir_text();
    assert!(text.contains(".equal "));
    assert!(parse_ir(&text.replace(".equal ", ".greater ")).is_err());
}

#[test]
fn allocation_precedes_element_effects_and_append_preserves_source_order() {
    let f = compile_sources(&["module M { function effect(n as Toybox.Lang.Number) as Toybox.Lang.Number { return n; } } function f() as Toybox.Lang.Array<Toybox.Lang.Number> { return [M.effect(1), M.effect(2)]; }"]).unwrap();
    let text = f.to_ir_text();
    let f = parse_unit_ir(&text).unwrap();
    assert_eq!(f.to_ir_text(), text);
    let single =
        compile("function f(n as Toybox.Lang.Number) as Toybox.Lang.Array<Toybox.Lang.Number> { return [n, 2]; }")
            .unwrap();
    assert!(matches!(
        single.operations()[0].operation,
        Operation::ArrayBegin {
            element_type: ArrayElement::Number,
            ..
        }
    ));
    assert!(text.find(".array-begin").unwrap() < text.find(".call ").unwrap());
}
