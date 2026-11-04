use ciq_compiler::{compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn class_calls_preserve_saved_stages_and_complete_application() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-class-calls/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".call-class "));
    assert!(text.contains(".call-class-discard Cold clear"));
    assert!(text.contains(" Nested.Methods twice "));
    assert!(
        text.lines()
            .any(|l| l.starts_with(".call-class ") && l.contains("capture=&"))
    );
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0xb67c1691c9a5422a9d3202a0c016870cu128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    for source in [
        "class C{static function f(){return 1;}static function g(){return C.f();}}function run(){C.f();}",
        "using M as Alias;module M{class C{static function f(){return 1;}}}function run(){return Alias.C.f()+$.M.C.f();}",
        "class C{static function f(){return self as Toybox.Lang.Object;}}function run(){return C.f();}",
        "class C{static function f(){return [1,2];}}function run(){return C.f()[0];}",
        "class C{static function f(n as Toybox.Lang.Number) as Void{}}function run(flag as Toybox.Lang.Boolean){C.f(flag?1:2);}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}")));
    }
}

#[test]
fn class_calls_require_static_targets_and_checked_signatures() {
    for source in [
        "class C{function f(){}}function run(){C.f();}",
        "class C{}function run(){C.missing();}",
        "class C{static function f(n as Toybox.Lang.Number){}}function run(){C.f();}",
        "class C{static function f(n as Toybox.Lang.Number){}}function run(){C.f(\"bad\");}",
        "class C{static function f(){}}function run(){var C=1;C.f();}",
        "class C{static var S as Toybox.Lang.String=read();static function read() as Toybox.Lang.String{return S;}}function run(){C.read();}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}

#[test]
fn saved_class_calls_validate_owner_signature_and_capture_lifetime() {
    let text = compile_sources(&["class C{static function f(n as Toybox.Lang.Number) as Toybox.Lang.Number{return n;}}function run(flag as Toybox.Lang.Boolean){return C.f(flag?1:2);}"])
        .unwrap().to_ir_text().lines().map(|l|l.split(" span=").next().unwrap()).collect::<Vec<_>>().join("\n");
    let begin = text
        .lines()
        .find(|l| l.starts_with(".begin-call "))
        .unwrap();
    let capture = format!(" capture={}", begin.strip_prefix(".begin-call ").unwrap());
    for bad in [
        text.replace(" C f ", " Missing f "),
        text.replace(" C f ", " C missing "),
        text.replace(begin, ""),
        text.replace(&capture, ""),
        text.replace(&capture, " capture=&999"),
        text.replace(begin, &format!("{begin}\n{begin}")),
        text.replace("receiver=static", "receiver=instance"),
        text.replace(" C f %", " C f result=string %"),
    ] {
        assert_ne!(bad, text);
        assert!(parse_unit_ir(&bad).is_err(), "{bad}");
    }
}

#[test]
fn class_call_arguments_follow_strict_checking_policy() {
    let source = "class C{static function f(n as Toybox.Lang.Number){return n;}}function run(n){return C.f(n);}";
    assert!(compile_sources(&[source]).is_ok());
    assert!(checking::compile_sources(&[source]).is_err());
}
