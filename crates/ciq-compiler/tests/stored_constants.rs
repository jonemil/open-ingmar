use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;
#[test]
fn stored_class_constants_keep_all_binary_and_saved_stages() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-class-constants/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".field-constant TEXT string"));
    assert!(text.contains("__ciq_initialize_fields"));
    let options = ciq_compiler::application::ConsoleApplication {
        id: 0xe20d80cce2c14eb09170d456824b4281u128.to_be_bytes(),
        entry: "MinimalConsoleApp.onStart".into(),
    };
    common::application(&unit, &options);
}
#[test]
fn constant_writes_and_allocation_metadata_are_checked() {
    for source in [
        "class C{const X=[1];function f(){self.X=[2];}}",
        "class C{const X=[1];function initialize(){self.X=[2];}}",
        "class C{const X=[1];}function f(x as C){x.X=[2];}",
        "class C{const X=1;function f(){X++;}}",
        "class C{const X=1;function f(){X+=1;}}",
        "class C{const X=1;function f(){self.__ciq_initialize_fields();}}",
        "class C{const A=B;const B=A;}",
        "class C{const X=missing;}",
        "class C{const X=self.Y; var Y as Toybox.Lang.Number; function initialize(){self.Y=1;}}",
        "class C{const X=self.f();function f() as Toybox.Lang.Number{return 1;}}",
        "function stop() as Toybox.Lang.Number{Toybox.System.exit();return 1;} class C{const X=stop();}",
        "class C{const X=true ? 1 : 2;}",
        "typedef T as interface {var X as Toybox.Lang.Number;}; class C{const X=1;} function set(x as T){x.X=2;} function f(){set(new C());}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let unit =
        compile_sources(&["class C{const X=1;function value() as Toybox.Lang.Number{return X;}}"])
            .unwrap();
    let text = unit.to_ir_text();
    let removed = text
        .lines()
        .filter(|line| !line.starts_with(".assign-self X "))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(parse_unit_ir(&removed).is_err());
    let mut forged = text.clone();
    let at = forged.find(".assign-self X ").unwrap();
    forged.insert_str(at, ".integer %10000 3\n.assign-self X %10000\n");
    assert!(parse_unit_ir(&forged).is_err());
    let mut fields_only = text.clone();
    let start = fields_only
        .find(".function __ciq_initialize_fields")
        .unwrap();
    let end = start + fields_only[start..].find(".end\n").unwrap() + 5;
    fields_only.replace_range(start..end, "");
    assert!(parse_unit_ir(&fields_only).is_err());
    let mut classes = unit.classes().to_vec();
    classes[0].fields[0].kind = ciq_compiler::FieldKind::Constant {
        value_type: ciq_compiler::ReturnType::Void,
    };
    assert!(
        ciq_compiler::CompilationUnit::with_classes(unit.declarations().to_vec(), classes).is_err()
    );
}

#[test]
fn private_source_allocations_require_matching_completion() {
    for body in [
        ".begin-source &0 C Lang.Object\n.return-void",
        ".new-source %0 C Lang.Object &0\n.return %0",
        ".begin-source &0 C Lang.Object\n.new-source %0 D Lang.Object &0\n.return %0",
        ".begin-source &0 C Lang.Object\n.new-source %0 C WatchUi.View &0\n.return %0",
        ".begin-source &0 C Lang.Object\n.new-source %0 C Lang.Object &0\n.discard %0\n.new-source %1 C Lang.Object &0\n.return %1",
        ".begin-source &0 C Lang.Object\n.array-finish %0 &0\n.return %0",
    ] {
        let text = format!("ciq-ir\n.function f\n{body}\n.end\n");
        assert!(ciq_compiler::parse_function_ir(&text).is_err(), "{text}");
    }
}
