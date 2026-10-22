use ciq_compiler::{
    ArrayElement, Operation,
    api::{ApiContext, ApiLevel},
    compile_function, compile_function_with_api, parse_function_ir, parse_function_ir_with_api,
};

#[test]
fn sized_allocations_preserve_nullable_contracts_and_native_lowering() {
    for (name, element) in [
        ("Number", ArrayElement::NullableNumber),
        ("Char", ArrayElement::NullableChar),
        ("String", ArrayElement::NullableString),
        ("Method", ArrayElement::NullableMethod),
        ("WeakReference", ArrayElement::NullableWeakReference),
        ("Boolean", ArrayElement::NullableBoolean),
        ("Long", ArrayElement::NullableLong),
        ("Float", ArrayElement::NullableFloat),
        ("Double", ArrayElement::NullableDouble),
        ("Symbol", ArrayElement::NullableSymbol),
    ] {
        let f = compile_function(&format!(
            "function f(n) as Toybox.Lang.Array<Toybox.Lang.{name} or Null> {{ return new[n]; }}"
        ))
        .unwrap();
        assert!(f.operations().iter().any(|o| matches!(o.operation,
            Operation::ArrayAllocate { ref element_type, .. } if *element_type == element)));
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        let assembly = f.lower();
        assert_eq!(
            assembly,
            ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
        );
        // argc/local header, Number parameter read, newa, ret: no append loop.
        assert_eq!(assembly.encode().unwrap(), [0x35, 2, 0x12, 1, 0x14, 0x16]);
    }
}

#[test]
fn source_and_saved_ir_cannot_hide_null_slots_or_invalid_length_types() {
    for source in [
        "function f() { return new[3]; }",
        "function f() { var a = new[3]; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number> { return new[3]; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number> { return new[0]; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return new[null]; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return new[true]; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return new[1l]; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return new[1.0]; }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return new[n]; }",
        "function f() as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return new[]; }",
    ] {
        assert!(compile_function(source).is_err(), "{source}");
    }
    let source =
        "function f(n) as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return new[n]; }";
    let text = compile_function(source).unwrap().to_ir_text();
    let changed = text.replace("nullable-number", "number");
    assert_ne!(changed, text);
    assert!(parse_function_ir(&changed).is_err());
    let changed = text
        .lines()
        .map(|line| {
            if line.starts_with(".array-allocate ") {
                let mut fields = line.split_whitespace().collect::<Vec<_>>();
                fields[3] = "%999";
                fields.join(" ")
            } else {
                line.into()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(parse_function_ir(&changed).is_err());
    // Keep invalid runtime sizes visible rather than clamping or dropping them.
    assert!(
        compile_function(
            "function f() as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return new[-1]; }"
        )
        .is_ok()
    );
}

#[test]
fn allocation_checks_element_availability_without_native_method_requirements() {
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let f = compile_function_with_api(
        "function f(n) as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return new[n]; }",
        initial,
    )
    .unwrap();
    assert_eq!(
        f,
        parse_function_ir_with_api(&f.to_ir_text(), initial).unwrap()
    );
    for name in ["Char", "WeakReference"] {
        let source = format!(
            "function f(n) as Toybox.Lang.Array<Toybox.Lang.{name} or Null> {{ return new[n]; }}"
        );
        assert!(compile_function_with_api(&source, initial).is_err());
        let f = compile_function(&source).unwrap();
        assert!(parse_function_ir_with_api(&f.to_ir_text(), initial).is_err());
    }
}

#[test]
fn conditional_allocations_and_lengths_keep_saved_ir_effect_order() {
    for source in [
        "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Array<Toybox.Lang.Method or Null> { return flag ? new[2] : new[0]; }",
        "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return flag ? new[2] : new[0]; }",
        "function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return new[flag ? 2 : 0]; }",
        "function f(n as Toybox.Lang.Number or Null) as Toybox.Lang.Array<Toybox.Lang.Number or Null> { if (n != null) { return new[n]; } return new[0]; }",
    ] {
        let f = compile_function(source).unwrap();
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
        f.lower().encode().unwrap();
    }
    let source = "function f() as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return new[2]; }";
    let text = compile_function(source).unwrap().to_ir_text();
    let changed = text.replace(".integer %0 2", ".boolean %0 true");
    assert_ne!(changed, text);
    assert!(parse_function_ir(&changed).is_err());
    // A definition cannot be moved after the allocation that consumes it.
    let mut lines = text.lines().map(str::to_owned).collect::<Vec<_>>();
    let length = lines
        .iter()
        .position(|l| l.starts_with(".integer "))
        .unwrap();
    lines.swap(length, length + 1);
    assert!(parse_function_ir(&lines.join("\n")).is_err());
}
