use ciq_compiler::{
    ArrayConstantValue, ArrayElement, Operation, ReturnType,
    api::{ApiContext, ApiLevel, Type},
    compile, compile_sources, compile_with_api, parse_ir, parse_ir_with_api, parse_unit_ir,
};

#[test]
fn nullable_elements_preserve_native_and_saved_pipelines() {
    let source = include_str!("../../../examples/compiler-nullable-arrays/app.mc");
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    for (element, expected, literal) in [
        ("Number", ArrayElement::NullableNumber, "[1, null, 2]"),
        (
            "String",
            ArrayElement::NullableString,
            "[\"one\", null, \"one\"]",
        ),
        ("String", ArrayElement::NullableString, "[null]"),
        ("Number", ArrayElement::NullableNumber, "[1]"),
        (
            "Boolean",
            ArrayElement::NullableBoolean,
            "[false, null, true]",
        ),
        ("Char", ArrayElement::NullableChar, "['A', null, 'C']"),
        ("Boolean", ArrayElement::NullableBoolean, "[null]"),
        ("Char", ArrayElement::NullableChar, "[null]"),
    ] {
        let f = compile(&format!("function f() as Toybox.Lang.Array<Toybox.Lang.{element} or Null> {{ return {literal}; }}")).unwrap();
        assert_eq!(f.return_type(), ReturnType::Array(expected));
        assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
        assert!(matches!(
            f.operations()[0].operation,
            Operation::ArrayConstant { .. }
        ));
        let mut malformed = f.operations().to_vec();
        let Operation::ArrayConstant {
            element_type,
            values,
            ..
        } = &mut malformed[0].operation
        else {
            unreachable!()
        };
        *element_type = ArrayElement::Char;
        values[0] = ArrayConstantValue::Null;
        assert!(ciq_compiler::Function::new("bad".into(), malformed).is_err());
    }
}

#[test]
fn nullable_boolean_char_arrays_keep_element_and_proof_contracts() {
    for (ty, literal, other, consume, tag) in [
        ("Boolean", "false", "'A'", "!value", "boolean"),
        ("Char", "'A'", "false", "value + 1", "char"),
    ] {
        let source = format!(
            "module A {{ function take(a as Toybox.Lang.Array<Toybox.Lang.{ty} or Null>) as Toybox.Lang.Array<Toybox.Lang.{ty} or Null> {{ a.add(null).addAll([{literal}, null]); return a.reverse().slice(null, null); }} function f(p as Toybox.Lang.Boolean) {{ var a = A.take([p ? {literal} : null, null]); return a.indexOf(null); }} }}"
        );
        let unit = compile_sources(&[&source]).unwrap();
        let text = unit.to_ir_text();
        assert_eq!(unit, parse_unit_ir(&text).unwrap());
        let edited = text.replace(
            &format!("take result=nullable-{tag}-array"),
            &format!("take result={tag}-array"),
        );
        assert_ne!(text, edited);
        assert!(parse_unit_ir(&edited).is_err());
        for body in [
            format!(
                "var value = a[0]; if (value == null) {{ return {literal}; }} a[0] = null; return {consume};"
            ),
            format!(
                "var value = a[0]; while (value == null) {{ value = {literal}; }} return {consume};"
            ),
        ] {
            let source = format!(
                "function f(a as Toybox.Lang.Array<Toybox.Lang.{ty} or Null>) as Toybox.Lang.{ty} {{ {body} }}"
            );
            let f = compile(&source).unwrap();
            let text = f.to_ir_text();
            assert_eq!(f, parse_ir(&text).unwrap());
        }
        for body in [
            "System.println(a[0]);".to_owned(),
            "if (a[0] != null) { System.println(a[0].toString()); }".to_owned(),
            format!("a.add({other});"),
            format!("a[0] = {other};"),
            format!("var b = [{literal}]; a.addAll(b);"),
        ] {
            let source = format!(
                "function f(a as Toybox.Lang.Array<Toybox.Lang.{ty} or Null>) {{ {body} }}"
            );
            assert!(compile(&source).is_err(), "accepted {source}");
        }
        for source in [
            format!(
                "function f(a as Toybox.Lang.Array<Toybox.Lang.{ty}>) as Toybox.Lang.Array<Toybox.Lang.{ty} or Null> {{ return a; }}"
            ),
            format!(
                "function f() as Toybox.Lang.Array<Toybox.Lang.{ty} or Null> {{ return [{other}, null]; }}"
            ),
            format!("function f() {{ var a = [{literal}, null]; }}"),
        ] {
            assert!(compile(&source).is_err(), "accepted {source}");
        }
    }
}

#[test]
fn nullable_boolean_char_array_api_checks_include_elements_and_consumers() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let char_api = ApiContext::new(ApiLevel(1, 3, 0), ApiLevel(1, 3, 0)).unwrap();
    for (ty, initial_ok) in [("Boolean", true), ("Char", false)] {
        let source = format!(
            "function f(a as Toybox.Lang.Array<Toybox.Lang.{ty} or Null>) {{ return a.size(); }}"
        );
        let f = compile(&source).unwrap();
        assert_eq!(compile_with_api(&source, initial).is_ok(), initial_ok);
        assert_eq!(
            parse_ir_with_api(&f.to_ir_text(), initial).is_ok(),
            initial_ok
        );
        assert!(compile_with_api(&source, char_api).is_ok());
        let source = source.replace("a.size()", "a.indexOf(null)");
        assert!(compile_with_api(&source, initial).is_err());
        assert!(compile_with_api(&source, char_api).is_ok());
    }
}

#[test]
fn reads_require_fresh_scalar_proofs_and_array_elements_remain_invariant() {
    for source in [
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number or Null>) { return a[0] + 1; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number or Null>) { a[0] = 1; return a[0] + 1; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number or Null>) { if (a[0] != null) { return a[0] + 1; } return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number or Null>) { var n = a[0]; if (n != null) { n = null; return n + 1; } return 0; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.String or Null>) { return a[0].length(); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.String or Null>) { a[0] = 1; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number or Null>) { a[0] = 'a'; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.String>) { a[0] = null; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return a; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.String or Null>) as Toybox.Lang.Array<Toybox.Lang.String> { return a; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return [\"bad\", null]; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return ['a']; }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Char or Null>) { return a[0].toNumber(); }",
        "function f(a as Toybox.Lang.Array<Toybox.Lang.Boolean or Null>) { return !a[0]; }",
        "function f() { var a = [null]; }",
        "function f() { var a = [1, null]; }",
    ] {
        assert!(compile(source).is_err(), "accepted {source}");
    }
    let source = "function f(a as Toybox.Lang.Array<Toybox.Lang.Number or Null>) { var n = a[0]; if (n != null) { a[0] = null; return n + 1; } return 0; }";
    let f = compile(source).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    assert!(parse_ir(&f.to_ir_text().replace(".not-equal", ".equal")).is_err());
}

#[test]
fn computed_context_and_native_arguments_keep_exact_element_contracts() {
    let source = "module A { function take(a as Toybox.Lang.Array<Toybox.Lang.String or Null>) as Toybox.Lang.Array<Toybox.Lang.String or Null> { a.add(null).addAll([\"two\", null]); return a; } function f(b as Toybox.Lang.Boolean) { var a = A.take([b ? \"one\" : null, null]); return a.size(); } }";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert!(
        compile_sources(&[&source.replace("A.take([b ? \"one\" : null, null])", "A.take([1])")])
            .is_err()
    );
    let source = "function f(a as Toybox.Lang.Array<Toybox.Lang.Number or Null>, n as Toybox.Lang.Number or Null) { a[0] = n; a.add(n); return a.indexOf(null); }";
    let f = compile(source).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let text = f.to_ir_text();
    assert!(parse_ir(&text.replace("nullable-number-array", "number-array")).is_err());
    for bad in ["a.add(\"x\");", "var other = [1]; a.addAll(other);"] {
        assert!(compile(&source.replace("a.add(n);", bad)).is_err());
    }
    let receiver = Type::Array(Box::new(ArrayElement::NullableNumber.api_type()));
    let reverse = ciq_compiler::api::core()
        .method_for_type(&receiver, "reverse")
        .unwrap();
    assert_eq!(reverse.result, receiver);
}

#[test]
fn nullable_storage_preserves_api_and_frame_checks() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let source =
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return [1, null]; }";
    let f = compile_with_api(source, initial).unwrap();
    assert_eq!(f, parse_ir_with_api(&f.to_ir_text(), initial).unwrap());
    let later_call = "function f(a as Toybox.Lang.Array<Toybox.Lang.Number or Null>) { a.add(null); return a.size(); }";
    assert!(compile_with_api(later_call, initial).is_err());
    let text = "ciq-ir\n.function f parameters=254\n.array-begin &0 nullable-number 2\n.integer %0 1\n.null %1\n.array-fill-many &0 0 %0 %1\n.array-finish %2 &0\n.return %2\n.end\n";
    let f = parse_ir(text).unwrap();
    assert_eq!(&f.lower().encode().unwrap()[..3], &[0x4a, 255, 1]);
    for bad in [
        text.replace("parameters=254", "parameters=255"),
        text.replace("nullable-number", "number"),
        text.replace("nullable-number", "nullable-string"),
        text.replace(".null %1", ".char %1 0x00000061"),
    ] {
        assert!(parse_ir(&bad).is_err(), "accepted {bad}");
    }
}
