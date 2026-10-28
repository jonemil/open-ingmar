use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;

#[test]
fn branching_initializers_keep_saved_stages_and_native_contracts() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-branch-initializers/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".shared-constant M FORMAT number-format"));
    assert!(text.contains(".shared-constant $ EARLY null"));
    assert!(text.contains(".branch "));
    assert!(text.contains(".block-value "));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x8c81c02fe8af422d8535691cf0540721u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
}

/// Replace only the reserved initializer body, leaving declaration and read
/// contracts intact so malformed cases reach unit-level initialization proofs.
fn with_body(body: &str) -> String {
    let mut text = compile_sources(&["const VALUE=+1;function f(){return VALUE;}"])
        .unwrap()
        .to_ir_text();
    let header = text
        .find(".function __ciq_initialize_shared_VALUE")
        .unwrap();
    let start = header + text[header..].find('\n').unwrap() + 1;
    let end = start + text[start..].find(".end\n").unwrap();
    text.replace_range(start..end, body);
    text
}

#[test]
fn writes_are_checked_by_execution_paths_instead_of_record_order() {
    for body in [
        ".block ^0\n.boolean %0 true\n.branch %0 ^1 ^2\n.block ^1\n.integer %1 7\n.assign-shared $ VALUE %1\n.jump ^3\n.block ^2\n.integer %2 8\n.assign-shared $ VALUE %2\n.jump ^3\n.block ^3\n.return-void\n",
        ".block ^0\n.jump ^2\n.block ^1\n.read-shared %1 $ VALUE result=integer\n.discard %1\n.return-void\n.block ^2\n.integer %0 7\n.assign-shared $ VALUE %0\n.jump ^1\n.block ^3\n.integer %2 9\n.assign-shared $ VALUE %2\n.return-void\n",
        ".block ^0\n.boolean %0 true\n.branch %0 ^0 ^1\n.block ^1\n.integer %1 7\n.assign-shared $ VALUE %1\n.return-void\n",
    ] {
        common::pipeline(parse_unit_ir(&with_body(body)).unwrap());
    }
}

#[test]
fn bypassed_repeated_and_unreachable_assignments_cannot_prove_initialization() {
    for (body, expected) in [
        (
            ".block ^0\n.boolean %0 true\n.branch %0 ^1 ^2\n.block ^1\n.integer %1 7\n.assign-shared $ VALUE %1\n.return-void\n.block ^2\n.return-void\n",
            "every returning path",
        ),
        (
            ".block ^0\n.boolean %0 true\n.branch %0 ^1 ^2\n.block ^1\n.integer %1 7\n.assign-shared $ VALUE %1\n.jump ^3\n.block ^2\n.jump ^3\n.block ^3\n.integer %2 8\n.assign-shared $ VALUE %2\n.return-void\n",
            "more than once",
        ),
        (
            ".block ^0\n.integer %0 7\n.assign-shared $ VALUE %0\n.jump ^0\n.block ^9\n.integer %9 1\n.assign-shared $ VALUE %9\n.return-void\n",
            "more than once",
        ),
        (
            ".block ^0\n.jump ^0\n.block ^1\n.integer %0 7\n.assign-shared $ VALUE %0\n.return-void\n",
            "entry-reachable",
        ),
        (
            ".block ^0\n.integer %0 7\n.assign-shared $ VALUE %0\n.return-void\n.block ^9\n.return-void\n",
            "every returning path",
        ),
        (
            ".block ^0\n.integer %0 7\n.assign-shared $ VALUE %0\n.read-shared %1 $ VALUE result=null\n.discard %1\n.return-void\n",
            "current slot state",
        ),
        (
            ".block ^0\n.boolean %0 true\n.branch %0 ^1 ^2\n.block ^1\n.integer %1 7\n.assign-shared $ VALUE %1\n.jump ^3\n.block ^2\n.jump ^3\n.block ^3\n.read-shared %2 $ VALUE result=null\n.discard %2\n.jump ^4\n.block ^4\n.jump ^4\n.block ^9\n.integer %9 9\n.assign-shared $ VALUE %9\n.return-void\n",
            "definite initialized",
        ),
    ] {
        let error = parse_unit_ir(&with_body(body)).unwrap_err();
        assert!(error.message.contains(expected), "{expected}: {error}");
    }
}

#[test]
fn branches_keep_owner_reentry_shutdown_and_type_checks() {
    for source in [
        "const X=!(false && true);function f(){return X;}",
        "const X=(true ? 1:2)+3;function f(){return X;}",
        "const X=(true ? 1:2) as Toybox.Lang.Number;function f(){return X;}",
        "const X=true ? [1]:[2];function f(){return X;}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap());
    }
    for source in [
        "const X=true?f():1;function f() as Toybox.Lang.Number{return 1;}",
        "module T{function stop() as Toybox.Lang.Number{Toybox.System.exit();return 1;}}const X=true?T.stop():1;function f(){}",
        "const X=1 ? 2:3;function f(){}",
        "const X=true && 1;function f(){}",
        "class C{const X=true ? [1]:[2];}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}
