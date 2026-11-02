use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;

#[test]
fn owner_receivers_preserve_identity_lookup_and_complete_saved_stages() {
    let source = include_str!("../../../examples/compiler-owner-receivers/app.mc");
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains("via=self.Target.Deep"));
    assert!(text.contains("class=Token@Lang.Object"));
    assert!(text.contains("lookup=lexical"));
    let assembly = unit.lower().to_text();
    assert!(assembly.contains(
        "getselfv @__ciq_member_Target\ngetsv @__ciq_member_Deep\ngetsv @__ciq_member_value"
    ));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0xb67c1691c9a5422a9d3202a0c016870cu128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    for bad in [
        text.replace("via=self.Target.Deep", "via=self.Missing.Deep"),
        text.replace("via=self.Target.Deep", "via=self.Target.Missing"),
        text.replace(
            "via=self.Target.Deep",
            "lookup=lexical via=self.Target.Deep",
        ),
    ] {
        assert_ne!(text, bad);
        assert!(parse_unit_ir(&bad).is_err());
    }
}

#[test]
fn me_and_self_share_instance_and_owner_qualification() {
    for source in [
        "const X=7;function f(){return me.X+self.X;}",
        "function value(){return 7;}function f(){return me.value()+self.value();}",
        "function value(){return 7;}module M{function f(){var value=1;return self.value()+me.value();}}",
        "module M{module Child{function f(){return 1;}}function f(){var Child=7;return me.Child.f();}}",
        "module M{var X=1;function f(){me.X++;self.X+=2;return me.X;}}",
        "var X=1;class C{function f(){me.X=2;return self.X;}}",
        "class C{var X as Toybox.Lang.Number;function initialize(){me.X=1;}function identity() as C{return me;}}",
        "module Child{function f(){return 1;}}class C{function f(){return me.Child.f();}}",
    ] {
        let unit = compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}"));
        common::pipeline(unit.clone());
        assert_eq!(
            unit.lower(),
            compile_sources(&[&source
                .replace("me.", "self.")
                .replace("return me;", "return self;")])
            .unwrap()
            .lower()
        );
    }
}

#[test]
fn receiver_checks_reject_early_exposure_missing_members_and_namespace_forgery() {
    for source in [
        "class C{var X as Toybox.Lang.Number;function initialize(){System.println(me);X=1;}}",
        "class C{var X as Toybox.Lang.Number;function initialize(){var m=me.method(:value);X=1;}function value(){return 1;}}",
        "function f(){return me.missing();}",
        "module M{function f(){return self.missing();}}",
        "function f(){var me=1;return me;}",
        "function f(){return me;}",
        "var Object=1;class C{function f(){return me.Object;}}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let source = "module Child{function f(){return 1;}}class C{var Child=1;function f(){return 1;}}function f(){return self.Child.f();}";
    let text = compile_sources(&[source])
        .unwrap()
        .to_ir_text()
        .lines()
        .map(|line| line.split(" span=").next().unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    // Move the valid global path call into a class body: its field shadows the namespace.
    let bad = text.replace(
        ".integer %0 1\n.return %0",
        ".call %0 Child f via=self.Child\n.return %0",
    );
    assert_ne!(bad, text);
    assert!(parse_unit_ir(&bad).is_err());
}

#[test]
fn namespace_selection_effects_are_checked_before_field_completion() {
    let source = "using Toybox.Lang;module M{var MARK as Lang.Number=1;module Target{function value() as Lang.Number{return 1;}}module Peer{function run() as Lang.Number{return self.Target.value();}}}class C{var value as Lang.Number=M.Peer.run();}";
    let text = compile_sources(&[source]).unwrap().to_ir_text();
    let mut lines = Vec::new();
    for line in text.lines() {
        lines.push(line);
        if line.starts_with(".assign-shared M MARK ") {
            lines.push(".call-native-module-discard Toybox.System exit");
        }
    }
    let bad = lines.join("\n");
    assert_ne!(bad, text.trim_end());
    let error = parse_unit_ir(&bad).unwrap_err();
    assert!(error.to_string().contains("shutdown"), "{error}");
    let source = "module M{module Child{function value(n as Toybox.Lang.Number){return n;}}function run(flag as Toybox.Lang.Boolean){return self.Child.value(flag?1:2);}}";
    let error = compile_sources(&[source]).unwrap_err();
    assert!(error.to_string().contains("call-target capture"), "{error}");
}
