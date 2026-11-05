use ciq_compiler::{Visibility, compile_sources, parse_unit_ir};
mod common;

#[test]
fn access_metadata_survives_all_saved_stages_and_prg_generation() {
    let unit =
        compile_sources(&[include_str!("../../../examples/compiler-visibility/app.mc")]).unwrap();
    assert_eq!(
        unit.member_visibility("Vault", "VALUE"),
        Visibility::Private
    );
    assert_eq!(
        unit.member_visibility("Vault", "HIDDEN"),
        Visibility::Protected
    );
    assert!(unit.to_ir_text().contains(".visibility VALUE private"));
    assert_eq!(unit.clone().with_visibility(Vec::new()).unwrap(), unit);
    let prg = common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: [7; 16],
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    let records =
        ciq_prg::data::classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
            .unwrap();
    let fields = records
        .iter()
        .filter_map(|r| {
            if let ciq_prg::data::classes::Record::Class(c) = r {
                Some(c)
            } else {
                None
            }
        })
        .flat_map(|c| &c.fields)
        .collect::<Vec<_>>();
    assert_eq!(
        fields
            .iter()
            .filter(|f| f.key & 0xff == 0x20 && f.kind == 1)
            .count(),
        2
    );
    assert!(
        fields
            .iter()
            .filter(|f| f.key & 0xff == 0x30 && f.kind == 6)
            .count()
            >= 2
    );
    assert_eq!(
        fields
            .iter()
            .filter(|f| f.key & 0xff == 0x10 && f.kind == 0)
            .count(),
        1
    );
    let lowered = unit.lower().to_text();
    for member in ["VALUE", "secret", "STATIC", "bump", "K"] {
        assert!(
            lowered.contains(&format!("__ciq_private_5_Vault_{member}")),
            "{member}"
        );
    }
}

#[test]
fn nonpublic_members_are_checked_across_files_and_access_forms() {
    let declarations = "class C{private var x=1;protected var y=2;hidden var z=3;private static var S=4;private const K=5;private enum{E=6}private function f(){return x;}protected static function sf(){return S;}}";
    for expression in [
        "c.x",
        "c.y",
        "c.z",
        "C.S",
        "c.K",
        "C.E",
        "c.f()",
        "C.sf()",
        "c.method(:f)",
        "c.method(:sf)",
        "c.x=2",
        "C.S=2",
    ] {
        let body = if expression.contains('=') {
            format!("{expression};")
        } else {
            format!("return {expression};")
        };
        let source = format!("function run(){{var c=new C();{body}}}");
        let public = declarations
            .replace("private ", "public ")
            .replace("protected ", "public ")
            .replace("hidden ", "public ");
        compile_sources(&[&public, &source])
            .unwrap_or_else(|e| panic!("public baseline {expression}: {e}"));
        assert!(
            compile_sources(&[declarations, &source]).is_err(),
            "{expression}"
        );
    }
    for source in [
        "private function f(){}",
        "module M{private var x=1;}",
        "private class C{}",
        "class C{public private var x=1;}",
        "class C{hidden protected function f(){}}",
        "class C{private function f(){return 1;}function g(){return method(:f);}}",
        "class C{protected function f(){return 1;}function g(){return method(:f);}}",
        "class C{var __ciq_private_1_C_x=1;}",
        "typedef I as interface{var x as Toybox.Lang.Number;};class C{private var x as Toybox.Lang.Number=1;}function take(c as I){return c.x;}function f(){return take(new C());}",
        "typedef I as interface{function read() as Toybox.Lang.Number;};class C{protected function read() as Toybox.Lang.Number{return 1;}}function take(c as I){return c.read();}function f(){return take(new C());}",
        "class C{static static function f(){}}",
        "class C{private function initialize(){}}function f(){return new C();}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}

#[test]
fn declaring_classes_can_use_nonpublic_members_and_private_constructors() {
    for source in [
        "class C{private var x=1;function f(other as C){return other.x;}}",
        "class C{protected var x=1;function f(other as C){return other.x;}}",
        "class C{private static var x=1;static private function f(){return C.x;}}",
        "class C{private function initialize(){}static function make(){return new C();}}",
        "class C{private function f(){return 1;}function g(){return self.f();}}",
        "public module M{public function f(){}}public class C{public var x=1;}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}")));
    }
}

#[test]
fn saved_visibility_records_reject_unknown_duplicate_or_late_targets() {
    let unit =
        compile_sources(&["class C{private var x=1;function f(){return x;}}function run(){}"])
            .unwrap();
    let text = unit.to_ir_text();
    for bad in [
        text.replace(".visibility x private", ".visibility missing private"),
        text.replace(
            ".visibility x private",
            ".visibility x private\n.visibility x public",
        ),
        text.replace(".visibility x private", ".visibility x invalid"),
    ] {
        assert!(parse_unit_ir(&bad).is_err(), "{bad}");
    }
}
