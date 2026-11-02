use ciq_compiler::{compile_sources, parse_unit_ir};
#[path = "common/checking.rs"]
mod checking;
mod common;

#[test]
fn staged_arguments_capture_targets_across_saved_stages() {
    let source = include_str!("../../../examples/compiler-call-captures/app.mc");
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".begin-call"));
    assert!(text.contains("capture=&"));
    assert!(text.contains("via=self.Target.Deep"));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0x6ed3112cf797467f85ee7525cb00da21u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
}

fn capture_text() -> String {
    compile_sources(&["module M{function value(n as Toybox.Lang.Number){return n;}}function run(flag as Toybox.Lang.Boolean){return M.value(flag?1:2);}"])
        .unwrap().to_ir_text().lines().map(|line|line.split(" span=").next().unwrap()).collect::<Vec<_>>().join("\n")
}

#[test]
fn saved_captures_require_one_active_completion_and_cannot_escape() {
    let text = capture_text();
    let begin = text
        .lines()
        .find(|line| line.starts_with(".begin-call"))
        .unwrap();
    let builder = begin.strip_prefix(".begin-call ").unwrap();
    let completion = format!(" capture={builder}");
    for bad in [
        text.replace(begin, ""),
        text.replace(&completion, ""),
        text.replace(begin, &format!("{begin}\n{begin}")),
        text.replace(begin, &format!("{begin}\n.return-void")),
        text.replace(&completion, " capture=&999"),
        text.replace(&completion, &format!(" span=0:1{completion}")),
        text.replace(&completion, &format!("{completion}{completion}")),
    ] {
        assert_ne!(bad, text);
        assert!(parse_unit_ir(&bad).is_err(), "{bad}");
    }
    let position = text.find(begin).unwrap();
    let mut bad = text.clone();
    bad.replace_range(position..position + begin.len(), "");
    let at = bad.find(&completion).unwrap();
    let at = bad[at..].find('\n').map(|n| at + n + 1).unwrap();
    bad.insert_str(at, &format!("{begin}\n"));
    assert!(parse_unit_ir(&bad).is_err());
}

#[test]
fn initialization_effects_happen_at_capture_before_argument_effects() {
    let source = "module M{var X as Toybox.Lang.Number=1;function value(n as Toybox.Lang.Number){return n;}}class C{var field as Toybox.Lang.Number;function initialize(){field=M.value(1);}}";
    let text = compile_sources(&[source]).unwrap().to_ir_text();
    let mut lines = text
        .lines()
        .map(|line| line.split(" span=").next().unwrap().to_owned())
        .collect::<Vec<_>>();
    let call = lines
        .iter()
        .position(|line| line.starts_with(".call ") && line.contains(" M value "))
        .unwrap();
    lines[call].push_str(" capture=&999");
    let argument = (0..call)
        .rev()
        .find(|&i| lines[i].starts_with(".integer "))
        .unwrap();
    lines.insert(argument, ".begin-call &999".into());
    let text = lines.join("\n");
    common::pipeline(parse_unit_ir(&text).unwrap());
    let bad = text
        .lines()
        .flat_map(|line| {
            if line.starts_with(".assign-shared M X ") {
                vec![line, ".call-native-module-discard Toybox.System exit"]
            } else {
                vec![line]
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let error = parse_unit_ir(&bad).unwrap_err();
    assert!(error.to_string().contains("shutdown"), "{error}");
}

#[test]
fn captures_reserve_both_frame_slots_and_release_after_invocation() {
    fn document(count: u32) -> String {
        let mut text = String::from("ciq-ir\n.function run\n");
        for builder in 0..count {
            text.push_str(&format!(
                ".begin-call &{builder}\n.call-global-discard target capture=&{builder}\n"
            ));
        }
        text.push_str(".return-void\n.end\n");
        text
    }
    let function = ciq_compiler::parse_function_ir(&document(127)).unwrap();
    let assembly = function.lower().to_text();
    assert_eq!(assembly.matches("frpush").count(), 127);
    assert_eq!(assembly.matches("npush").count(), 254);
    assert_eq!(
        function,
        ciq_compiler::parse_function_ir(&function.to_ir_text()).unwrap()
    );
    let error = ciq_compiler::parse_function_ir(&document(128)).unwrap_err();
    assert!(error.to_string().contains("255-slot"), "{error}");
}

#[test]
fn capture_setup_invalidates_mutable_fields_but_preserves_value_snapshots() {
    let prefix = "module M{function value(n as Toybox.Lang.Number){return n;}}";
    let stale = format!(
        "{prefix} class C{{var n as Toybox.Lang.Number or Null;function f(flag as Toybox.Lang.Boolean){{if(self.n!=null){{return M.value(flag?self.n:0);}}return 0;}}}}"
    );
    assert!(checking::compile_sources(&[&stale]).is_err());
    let snapshot = format!(
        "{prefix} class C{{var n as Toybox.Lang.Number or Null;function f(flag as Toybox.Lang.Boolean){{var saved=self.n;if(saved!=null){{return M.value(flag?saved:0);}}return 0;}}}}"
    );
    common::pipeline(checking::compile_sources(&[&snapshot]).unwrap());
    let record = "module M{function record(n as Toybox.Lang.Number) as {:value as Toybox.Lang.Number}{return {:value=>n};}}function f(flag as Toybox.Lang.Boolean){var r=M.record(flag?1:2);var n=r[:value];if(n!=null){return n;}return 0;}";
    common::pipeline(compile_sources(&[record]).unwrap());
}
