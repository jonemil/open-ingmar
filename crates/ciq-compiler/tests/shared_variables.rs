use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;
#[test]
fn shared_variables_preserve_contracts_owners_and_complete_pipeline() {
    let sources = [
        include_str!("../../../examples/compiler-shared-variables/app.mc"),
        include_str!("../../../examples/compiler-shared-variables/later.mc"),
    ];
    let unit = compile_sources(&sources).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".shared-variable $ GLOBAL any"));
    assert!(text.contains(".shared-variable M VALUE integer"));
    assert!(text.contains(".begin-shared "));
    assert!(text.contains(".write-shared "));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x9dc7a27b1eb34adba087012221feb6f6u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    common::pipeline(compile_sources(&[sources[1], sources[0]]).unwrap());
    for bad in [
        text.replace(
            ".shared-variable M VALUE integer",
            ".shared-constant M VALUE integer",
        ),
        text.replace(
            ".shared-variable M VALUE integer",
            ".shared-variable M VALUE string",
        ),
        text.replace(".shared-variable M VALUE integer\n", ""),
        text.replacen(
            ".begin-shared &0 M VALUE",
            ".begin-shared &0 Missing VALUE",
            1,
        ),
    ] {
        assert!(bad != text, "malformed case must change the unit");
        assert!(parse_unit_ir(&bad).is_err());
    }
}

#[test]
fn shared_variable_types_readonly_bindings_and_initialization_are_checked() {
    for source in [
        "using Toybox.Lang;var A as Lang.Number;function f(){}",
        "using Toybox.Lang;var A as Lang.Number=1;function f(){A=\"bad\";}",
        "using Toybox.Lang;module M{var A as Lang.Number=1;}function f(){M.A=\"bad\";}",
        "const A=1;function f(){A=2;}",
        "module M{const A=1;}function f(){M.A=2;}",
        "var A=1;const A=2;function f(){}",
        "enum Kind{A,B}var VALUE as Kind=A;function f(){VALUE=3;}",
        "module A{var X=B.X;}module B{var X=A.X;}function f(){return A.X;}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    for source in [
        "class Token{function initialize(){}function value(){return 4;}}var BOX as Token or Null;function f(){BOX=new Token();var x=BOX;if(x!=null){return x.value();}return 0;}",
        "enum Kind{A,B}var VALUE as Kind=A;function f(){VALUE=B;return VALUE;}",
        "var A=1;function f(){A=\"text\";return A.length();}",
        "var A;function f(){A=1;A=\"text\";return A;}",
        "using Toybox.Lang;var A as Lang.Number or Null;function f(){return A;}",
        "module M{var A=1;function f(){self.A++;self.A+=2;return self.A;}}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}")));
    }
}

#[test]
fn shared_stores_require_one_capture_and_one_completion_on_each_path() {
    for body in [
        ".begin-shared &0 M VALUE\n.return-void",
        ".integer %0 1\n.write-shared &0 %0\n.return-void",
        ".begin-shared &0 M VALUE\n.integer %0 1\n.write-shared &1 %0\n.return-void",
        ".begin-shared &0 M VALUE\n.integer %0 1\n.write-shared &0 %0\n.integer %1 2\n.write-shared &0 %1\n.return-void",
        ".begin-shared &0 M VALUE\n.begin-shared &0 M VALUE\n.integer %0 1\n.write-shared &0 %0\n.return-void",
    ] {
        let text = format!("ciq-ir\n.function invalid\n{body}\n.end\n");
        assert!(ciq_compiler::parse_function_ir(&text).is_err(), "{body}");
    }
}

#[test]
fn shared_store_target_validation_does_not_depend_on_block_text_order() {
    let unit = compile_sources(&["module M{var VALUE=1;}function f(){}"])
        .unwrap()
        .to_ir_text();
    let start = unit.find(".function f").unwrap();
    let end = start + unit[start..].find(".end").unwrap() + ".end".len();
    let body = ".function f\n.block ^2\n.jump ^0\n.block ^1\n.integer %0 1\n.write-shared &0 %0\n.return-void\n.block ^0\n.begin-shared &0 M VALUE\n.jump ^1\n.block ^3\n.return-void\n.end";
    let mut text = unit;
    text.replace_range(start..end, body);
    common::pipeline(parse_unit_ir(&text).unwrap());
    assert!(parse_unit_ir(&text.replace("&0 M VALUE", "&0 Missing VALUE")).is_err());
}
