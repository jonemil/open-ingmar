use ciq_compiler::{compile, compile_sources, parse_ir, parse_unit_ir};
use ciq_mc::Opcode;

const D: &str = "Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number>";

#[test]
fn indexed_reads_and_writes_have_dictionary_ir_and_direct_vm_operations() {
    let numeric = compile("function f(d as Toybox.Lang.Dictionary<Toybox.Lang.Number, Toybox.Lang.Number>, key as Toybox.Lang.Number) as Toybox.Lang.Number or Null { return d[key]; }").unwrap();
    assert_eq!(
        numeric.lower().encode().unwrap(),
        [0x35, 3, 0x12, 1, 0x12, 2, 0x10, 0x16]
    );
    let source = format!(
        "function f(d as {D}) as Toybox.Lang.Number or Null {{ d[\"two\"] = 42; return d[\"one\"]; }}"
    );
    let f = compile(&source).unwrap();
    let text = f.to_ir_text();
    assert!(text.contains(".dictionary-write"));
    assert!(text.contains(".dictionary-read"));
    assert!(!text.contains(".array-read"));
    assert_eq!(f, parse_ir(&text).unwrap());
    let p = f.lower();
    for expected in [Opcode::Agetv, Opcode::Aputv] {
        assert!(p.items().iter().any(
            |i| matches!(i, ciq_assembly::Item::Instruction { opcode, .. } if *opcode == expected)
        ));
    }
    assert_eq!(p, ciq_assembly::Program::parse(&p.to_text()).unwrap());
    assert!(parse_ir(&text.replace(".dictionary-write", ".array-write")).is_err());
    assert!(parse_ir(&text.replace(".dictionary-read", ".array-read")).is_err());
    assert!(
        parse_ir(&text.replace(
            "dictionary(string=>integer)",
            "dictionary(integer=>integer)"
        ))
        .is_err()
    );
}

#[test]
fn saved_ir_cannot_postpone_a_read_until_after_mutation() {
    let text = "ciq-ir\n.function f parameters=1 parameter-types=dictionary(integer=>integer) result=nullable-integer\n.parameter %0 0\n.integer %1 1\n.dictionary-read %2 %0 %1\n.parameter %3 0\n.integer %4 1\n.integer %5 42\n.dictionary-write %3 %4 %5\n.return %2\n.end\n";
    assert!(
        parse_ir(text)
            .unwrap_err()
            .message
            .contains("evaluation order")
    );
}

#[test]
fn lookup_proofs_and_mutation_survive_source_and_saved_units() {
    let source = format!(
        "module D {{
        function read(d as {D}) as Toybox.Lang.Number or Null {{ return d.get(\"one\"); }}
        function update(d as {D}) {{
            var copy = d;
            var value = D.read(d);
            if (value != null) {{ copy[\"two\"] = value + 1; }}
            d.put(\"three\", 3);
            d.remove(\"one\");
            var previous = d[\"two\"];
            d[\"two\"] = 41;
            if (previous != null) {{ return previous; }}
            return 0;
        }}
        function presence(d as {D}) as Toybox.Lang.Boolean {{ return d.hasKey(\"one\"); }}
    }}"
    );
    let unit = compile_sources(&[&source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert_eq!(
        unit.lower(),
        ciq_assembly::Program::parse(&unit.lower().to_text()).unwrap()
    );
    for (key, value, literal, body) in [
        (
            "Number",
            "String",
            "{1 => \"one\"}",
            "d[2] = \"two\"; var v = d.get(1); if (v != null) { return v.length(); }",
        ),
        (
            "Char",
            "Number",
            "{'a' => 1}",
            "d.put('b', 2); var v = d['a']; if (v != null) { return v + 1; }",
        ),
        (
            "Boolean",
            "String or Null",
            "{true => \"yes\", false => null}",
            "d[true] = null; var v = d.get(false); if (v != null) { return v.length(); }",
        ),
    ] {
        let source = format!(
            "module M {{ function make() as Toybox.Lang.Dictionary<Toybox.Lang.{key}, Toybox.Lang.{value}> {{ return {literal}; }} function run() as Toybox.Lang.Number {{ var d = M.make(); {body} return 0; }} }}"
        );
        let unit = compile_sources(&[&source]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
}

#[test]
fn nullable_values_require_guarded_uses_and_typed_keys_and_writes() {
    for body in [
        "return d[\"one\"] + 1;",
        "return d.get(\"one\");",
        "if (d.hasKey(\"one\")) { return d[\"one\"] + 1; } return 0;",
        "var value = d[\"one\"]; if (value != null) { value = null; return value + 1; } return 0;",
        "return d[1];",
        "return d.get(1);",
        "return d.hasKey(1);",
        "d[1] = 1; return 0;",
        "d[\"one\"] = \"bad\"; return 0;",
        "d[\"one\"] = null; return 0;",
        "d.put(\"one\", null); return 0;",
        "d.remove(1); return 0;",
        "return d.put(\"one\", 1);",
        "return d.remove(\"one\");",
        "d[\"one\"]++; return 0;",
    ] {
        let source = format!("function f(d as {D}) as Void {{ {body} }}");
        assert!(compile(&source).is_err(), "accepted {source}");
    }
    for body in [
        "return d[\"one\"];",
        "d[\"one\"] = 1; return 0;",
        "return d.get(\"one\");",
        "return d.hasKey(\"one\");",
    ] {
        let source = format!("function f(d as Toybox.Lang.Dictionary) as Void {{ {body} }}");
        assert!(compile(&source).is_err());
    }
    let nullable = "Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number or Null>";
    let source = format!(
        "function f(d as {nullable}) as Toybox.Lang.Boolean {{ d[\"one\"] = null; d.put(\"two\", null); return d.get(\"one\") == null; }}"
    );
    let f = compile(&source).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
}

#[test]
fn branches_in_write_values_stage_receiver_and_key_before_the_join() {
    let source = format!(
        "function f(d as {D}, choose as Toybox.Lang.Boolean) as Toybox.Lang.Number {{ d[\"one\"] = choose ? 1 : 2; var n = d[choose ? \"one\" : \"missing\"]; if (n != null) {{ return n; }} return 0; }}"
    );
    let f = compile(&source).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let text = f.to_ir_text();
    assert!(text.contains(".jump-value"));
    assert!(text.contains(".dictionary-write"));
    assert!(text.contains(".dictionary-read"));
}

#[test]
fn guarded_nullable_keys_preserve_candidate_lookup_types_without_bypassing_proofs() {
    let source = format!("function f(d as {D}, key as Toybox.Lang.String or Null) {{
        if (key != null) {{ var value = d[key]; if (value != null) {{ d[key] = value + 1; return value; }} }}
        return 0;
    }}");
    let f = compile(&source).unwrap();
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let unsafe_source = source.replace("if (key != null)", "if (true)");
    assert!(compile(&unsafe_source).is_err());
    let unsafe_ir = f.to_ir_text().replace(".dictionary-read", ".array-read");
    assert!(parse_ir(&unsafe_ir).is_err());
}
