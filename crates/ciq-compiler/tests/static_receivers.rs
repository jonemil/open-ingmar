use ciq_compiler::{compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn opaque_static_receivers_keep_origin_through_aliases_storage_and_annotations() {
    common::application(
        &compile_sources(&[include_str!(
            "../../../examples/compiler-class-calls/app.mc"
        )])
        .unwrap(),
        &ciq_compiler::application::ConsoleApplication {
            id: [7; 16],
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    for source in [
        "class C{static function owner() as C{return self;}}",
        "class C{static function owner() as C{var receiver=self;return receiver;}}",
        "class C{static const OWNER=self;static function owner() as C{return OWNER;}}",
        "class C{static var OWNER as C=self;static function owner() as C{return OWNER;}}",
        "class C{static function owner() as Toybox.Lang.Object{var receiver=self;return receiver;}}",
        "class C{static function take(x as C) as C{return x;}static function owner() as C{return take(self);}}",
        "class C{static const OWNERS=[self];static function owner() as C{return OWNERS[0];}}",
        "class C{static function owner(flag as Toybox.Lang.Boolean) as C{return flag?self:self;}}",
    ] {
        let unit = checking::compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert!(unit.to_ir_text().contains(".static-self"));
        common::pipeline(unit);
    }
}

#[test]
fn opaque_origins_do_not_grant_instance_or_module_proofs() {
    for source in [
        "class C{var value=1;static function bad(){var receiver=self;return receiver.value;}}",
        "class C{static function bad(){var receiver=self;return receiver.hashCode();}}",
        "class C{static const OWNER=self;static function bad(){return OWNER.hashCode();}}",
        "class D{}class C{static function bad() as D{return self;}}",
        "module M{}class C{static function bad() as C{return M;}}",
        "class C{static function bad() as C{return C;}}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}

#[test]
fn saved_static_receiver_tags_preserve_origin_and_reject_forgery() {
    let unit=compile_sources(&["class C{static const OWNER=self;static function owner() as C{return OWNER;}}function f(){}"]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains("static-receiver(C)"));
    common::pipeline(unit);
    for bad in [
        text.replace("static-receiver(C)", "module-value"),
        text.replace("static-receiver(C)", "static-receiver(D)"),
        text.replace("static-receiver(C)", "static-receiver()"),
    ] {
        assert!(parse_unit_ir(&bad).is_err(), "{bad}");
    }
}

#[test]
fn saved_origin_contracts_survive_parameters_and_returns() {
    let text = "ciq-unit\n.owner global\nciq-ir\n.function echo parameters=1 parameter-types=static-receiver(C) result=static-receiver(C)\n.parameter %0 0\n.return %0\n.end\n.class C Lang.Object\nciq-ir\n.function initialize receiver=instance\n.return-void\n.end\n.end-class\n";
    common::pipeline(parse_unit_ir(text).unwrap());
    assert!(
        parse_unit_ir(&text.replace("static-receiver(C)", "static-receiver(Missing)")).is_err()
    );
}
