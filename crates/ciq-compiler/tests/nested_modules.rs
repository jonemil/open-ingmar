use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;

#[test]
fn nested_names_reopened_files_and_metadata_survive_all_stages() {
    let sources = [
        include_str!("../../../examples/compiler-nested-modules/app.mc"),
        include_str!("../../../examples/compiler-nested-modules/later.mc"),
    ];
    let unit = compile_sources(&sources).unwrap();
    assert!(unit.modules().contains("Empty.Child"));
    let text = unit.to_ir_text();
    assert!(text.contains(".module Tree.Branch.Leaf"));
    assert!(text.contains("lookup=lexical"));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0xb67c1691c9a5422a9d3202a0c016870cu128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    common::pipeline(compile_sources(&[sources[1], sources[0]]).unwrap());
}

#[test]
fn nested_lookup_rejects_collisions_cycles_and_forged_ir() {
    for source in [
        "module A{module B{}const B=1;}",
        "module A{module B{}function B(){}}",
        "module A{module B{class C{class D{}}}}",
        "module A{const X=B.Y;module B{const Y=X;}}function f(){return A.X;}",
        "module A{const X=1;module B{const X=2;module C{function f(){return X;}}}}",
    ] {
        // The last source is valid; its saved IR must reject bypassing B.X.
        let result = compile_sources(&[source]);
        if source.contains("const X=2") {
            let text = result.unwrap().to_ir_text();
            let bad = text.replace(
                " A.B X result=integer lookup=lexical",
                " A X result=integer lookup=lexical",
            );
            assert_ne!(text, bad);
            assert!(parse_unit_ir(&bad).is_err());
        } else {
            assert!(result.is_err(), "{source}");
        }
    }
    let source = "module A{const X=1;module B{function f(){return X;}}}";
    let text = compile_sources(&[source]).unwrap().to_ir_text();
    assert!(
        parse_unit_ir(&text.replace(
            " A X result=integer lookup=lexical",
            " A.B X result=integer lookup=lexical"
        ))
        .is_err()
    );
    assert!(ciq_compiler::compile_function("module Empty{}function f(){}").is_err());
}

#[test]
fn lexical_modifier_is_not_a_string_token() {
    common::pipeline(compile_sources(&["function f(){return \"lookup=lexical\";}"]).unwrap());
}

#[test]
fn parent_calls_constructors_and_nearest_names_keep_their_lookup_mode() {
    for source in [
        "module A{function f(){return 1;}module B{function g(){return self.f();}}}",
        "class Token{} module A{module B{function f(){return new Token();}}}",
        "module A{module B{function f(){return 1;}}module C{function g(){return B.f();}}}",
        "module A{module B{}}function f(){return 1;}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}")));
    }
    for source in [
        "module A{module B{function f(){return 1;}}module C{const B=1;module D{function g(){return B.f();}}}}",
        "module A{function f(){return 1;}module B{module f{}module C{function g(){return f();}}}}",
        "module A{const X=B.Y;module B{const Y=X;}}function f(){return A.X;}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}
