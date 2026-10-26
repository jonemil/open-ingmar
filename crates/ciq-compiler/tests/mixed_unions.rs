#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{compile_sources, parse_unit_ir};
const CLASSES: &str = "class Token{function initialize(){} function count() as Toybox.Lang.Number{return 7;}} class Marker{function initialize(){}}";

#[test]
fn mixed_alternatives_preserve_signatures_joins_storage_and_both_guard_edges() {
    for body in [
        "function f(x as Token or Toybox.Lang.String) as Toybox.Lang.Number{if(x instanceof Token){return x.count();}return x.length();}",
        "function f(x as Token or Toybox.Lang.Number) as Toybox.Lang.Number{if(x instanceof Toybox.Lang.Number){return x+1;}return x.count();}",
        "function f(x as Null or Toybox.Lang.String or Token) as Toybox.Lang.Number{if(x==null){return 0;}if(!(x instanceof Token)){return x.length();}return x.count();}",
        "function f(b as Toybox.Lang.Boolean) as Token or Toybox.Lang.String{return b?new Token():\"hi\";}",
        "function f(b as Toybox.Lang.Boolean) as Toybox.Lang.Number{var x=new Token();if(b){x=\"hello\";}if(x instanceof Token){return x.count();}return x.length();}",
        "module M{function echo(x as Token or Toybox.Lang.String) as Token or Toybox.Lang.String{return x;}}function f() as Toybox.Lang.Number{var x=M.echo(\"hello\");if(x instanceof Token){return x.count();}return x.length();}",
        "function f(x as Token or Toybox.Lang.Object) as Toybox.Lang.Number{if(x instanceof Token){return x.count();}return 0;}",
        "function f(x as Token or Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number{if(x instanceof Token){return x.count();}return x[0];}",
    ] {
        let unit = compile_sources(&[CLASSES, body]).unwrap_or_else(|e| panic!("{body}: {e}"));
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
        assert_eq!(
            unit.lower(),
            ciq_assembly::Program::parse(&unit.lower().to_text()).unwrap()
        );
    }
}
#[test]
fn erased_native_values_cannot_claim_exact_class_alternatives() {
    for body in [
        "function f(x as Toybox.Lang.Object) as Token or Toybox.Lang.String{return x;}",
        "function f(x as Token or Toybox.Lang.String) as Token{return x;}",
        "function f(x as Token or Toybox.Lang.String){return x.count();}",
        "function f(x as Token or Toybox.Lang.String) as Token or Toybox.Lang.String{return new Marker();}",
        "function f(x as Token or Toybox.Lang.String) as Token or Toybox.Lang.String{return null;}",
        "function f(x as Token or Toybox.Lang.String){if(x instanceof Token){x=\"hello\";return x.count();}return 0;}",
    ] {
        assert!(
            checking::compile_sources(&[CLASSES, body]).is_err(),
            "{body}"
        );
    }
}

#[test]
fn structural_witnesses_are_checked_on_each_mixed_boundary() {
    let classes = "class Token{function initialize(){}} class Good{function initialize(){} function compareTo(other as Toybox.Lang.Object) as Toybox.Lang.Number{return 0;}} class Bad{function initialize(){}}";
    for body in [
        "function f(x as Good) as Token or Toybox.Lang.Comparable{return x;}",
        "function f(x as Token or Toybox.Lang.Comparable,y as Good) as Token or Toybox.Lang.Comparable{x=y;return x;}",
        "function f(x as Token or Toybox.Lang.Comparable,y as Good) as Token or Toybox.Lang.Comparable{var value=x;value=y;return value;}",
        "module M{function echo(x as Token or Toybox.Lang.Comparable) as Token or Toybox.Lang.Comparable{return x;}}function f(x as Good){return M.echo(x);}",
        "function f(x as Good or Toybox.Lang.String) as Toybox.Lang.Comparable{return x;}",
    ] {
        let unit = compile_sources(&[classes, body]).unwrap_or_else(|e| panic!("{body}: {e}"));
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
        assert!(
            compile_sources(&[classes, &body.replace("Good", "Bad")]).is_err(),
            "{body}"
        );
    }
    assert!(
        compile_sources(&[
            classes,
            "function f(x as Good or Toybox.Lang.ByteArray) as Toybox.Lang.Comparable{return x;}"
        ])
        .is_err()
    );
}

#[test]
fn saved_edges_cannot_forge_deferred_interface_witnesses() {
    let unit = compile_sources(&[
        CLASSES,
        "function f(b as Toybox.Lang.Boolean){var x=b?new Token():new Marker();return 0;}",
    ])
    .unwrap();
    let text = unit.to_ir_text();
    let changed = text
        .lines()
        .map(|line| {
            if line.starts_with(".block-value ") {
                line.replace(
                    "source-union(source(Marker@Lang.Object)|source(Token@Lang.Object))",
                    "source-union(source(Token@Lang.Object)|comparable)",
                )
            } else {
                line.into()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(text.trim(), changed.trim());
    assert!(parse_unit_ir(&changed).is_err());
}

#[test]
fn mixed_contract_normalization_retains_exact_and_opaque_alternatives() {
    let a=compile_sources(&[CLASSES,"function f(x as Toybox.Lang.String or Token or Null or Token) as Token or Toybox.Lang.String or Null{return x;}"]).unwrap();
    let b=compile_sources(&[CLASSES,"function f(x as Null or Token or Toybox.Lang.String) as Null or Toybox.Lang.String or Token{return x;}"]).unwrap();
    assert_eq!(
        a.declarations()[0].function.parameter_types(),
        b.declarations()[0].function.parameter_types()
    );
    assert_eq!(
        a.declarations()[0].function.return_type(),
        b.declarations()[0].function.return_type()
    );
    let text = a.to_ir_text();
    assert!(text.contains("source-union(source(Token@Lang.Object)|string|null)"));
    assert!(parse_unit_ir(&text.replace("|string|null)", "|void|null)")).is_err());
    let erased=compile_sources(&[CLASSES,"function f(x as Token or Toybox.Lang.Object) as Token{if(x instanceof Token){return x;}return new Token();}"]).unwrap();
    assert!(
        erased
            .to_ir_text()
            .contains("source-union(source(Token@Lang.Object)|object)")
    );
    assert_eq!(erased, parse_unit_ir(&erased.to_ir_text()).unwrap());
}
