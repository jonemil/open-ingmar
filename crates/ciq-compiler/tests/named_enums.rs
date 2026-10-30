use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;

#[test]
fn enum_calls_containers_and_fields_keep_the_complete_pipeline() {
    let sources = [
        include_str!("../../../examples/compiler-named-enums/app.mc"),
        include_str!("../../../examples/compiler-named-enums/later.mc"),
    ];
    let unit = compile_sources(&sources).unwrap();
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0xffb2aaae51f241399b9604e4f2c70721u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    common::pipeline(compile_sources(&[sources[1], sources[0]]).unwrap());
}

#[test]
fn nominal_values_survive_native_and_saved_ir() {
    let source = "using Toybox.Lang;enum Color{RED,GREEN=5,BLUE}function choose(value as Color) as Color{return value;}function number(value as Color) as Lang.Number{return value;}function text(){return BLUE.toString();}";
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".enum $ Color integer"));
    assert!(text.contains(".enum-value "));
    assert!(text.contains("enum(Color=integer)"));
    common::pipeline(unit);
    assert!(parse_unit_ir(&text.replace(".enum $ Color integer\n", "")).is_err());
}

#[test]
fn enum_boundaries_reject_numbers_other_identities_and_mutable_widening() {
    let prefix = "using Toybox.Lang;enum Color{RED,GREEN}enum Other{OTHER}";
    for body in [
        "function take(value as Color){}function f(){take(1);}",
        "function take(value as Color){}function f(){take(OTHER);}",
        "const X as Color=1;function f(){}",
        "const X as Color=OTHER;function f(){}",
        "function take(value as Color){}function f(){take(-RED);}",
        "function take(value as Color){}function f(){take(RED+0);}",
        "function take(value as Color){}function f(){var value=RED;value++;take(value);}",
        "function take(values as Lang.Array<Color>){}function f(){take([OTHER]);}",
        "function take(values as Lang.Array<Color>){}function f(){take([1]);}",
        "function take(values as Lang.Array<Lang.Number>){}function make() as Lang.Array<Color>{return [RED];}function f(){take(make());}",
        "function f(){return Color.RED;}",
        "function f(){RED=GREEN;}",
        "function f(){return new Color();}",
        "function f(){return RED instanceof Color;}",
    ] {
        let source = format!("{prefix}{body}");
        assert!(compile_sources(&[&source]).is_err(), "{body}");
    }
    for body in [
        "function take(value as Color) as Color{return value;}function f() as Color{return take(+RED);}",
        "function number(value as Lang.Number) as Lang.Number{return value;}function f(){return number(RED);}",
        "function f(){var value=RED;if(value instanceof Lang.Number){return value as Color;}return RED;}",
        "function f() as Lang.Dictionary<Color,Color>{return {RED=>GREEN};}",
        "function f(){var values=[RED,GREEN];values.add(RED);return values;}",
        "enum More{COPY=RED,BRANCH=true?1:2}function f() as More{return COPY;}",
    ] {
        let source = format!("{prefix}{body}");
        common::pipeline(compile_sources(&[&source]).unwrap());
    }
}

#[test]
fn saved_units_require_unique_enum_definitions_in_nested_contracts() {
    for operand in [".float %0 0x3f800000", ".null %0", ".boolean %0 true"] {
        let text = format!(
            "ciq-ir\n.function bad\n{operand}\n.enum-value %1 Color=integer %0\n.return %1\n.end\n"
        );
        assert!(ciq_compiler::parse_function_ir(&text).is_err(), "{operand}");
    }
    let text=compile_sources(&["using Toybox.Lang;enum Color{RED}function unused(a as Lang.Array<Color>){}function value() as Color{return RED;}"]).unwrap().to_ir_text();
    for forged in [
        text.replace(".enum $ Color integer\n", ""),
        text.replace(
            ".enum $ Color integer\n",
            ".enum $ Color integer\n.enum $ Color integer\n",
        ),
        text.replace(".enum $ Color integer", ".enum $ bad-name"),
        text.replace(
            "parameter-types=enum(Color=integer)-array",
            "parameter-types=enum(Missing=integer)-array",
        ),
        text.replace("Color=integer %", "Missing=integer %"),
    ] {
        assert_ne!(forged, text);
        assert!(parse_unit_ir(&forged).is_err());
    }
    let source = "enum Color{RED}enum Color{BLUE}function f(){}";
    assert!(compile_sources(&[source]).is_err());
    assert!(compile_sources(&["enum Color{RED}class Color{}function f(){}"]).is_err());
    assert!(compile_sources(&["enum Color{RED}const Color=1;function f(){}"]).is_err());
}
