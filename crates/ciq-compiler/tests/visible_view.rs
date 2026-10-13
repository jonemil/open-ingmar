use ciq_compiler::{
    ClassBase, NativeClass, ParameterType, ReturnType,
    application::{ConsoleApplication, build_console_application},
    compile_sources, parse_unit_ir,
};

const APP: &str = include_str!("../../../examples/compiler-visible-view/app.mc");
const VIEW: &str = include_str!("../../../examples/compiler-visible-view/view.mc");

#[test]
fn visible_view_preserves_contracts_and_complete_text_pipelines() {
    // Put the View first: application entry selection must use its base identity.
    let unit = compile_sources(&[VIEW, APP]).unwrap();
    let view = &unit.classes()[0];
    assert_eq!(view.base, ClassBase::WatchUiView);
    let update = view
        .methods
        .iter()
        .find(|f| f.name() == "onUpdate")
        .unwrap();
    assert_eq!(
        update.parameter_types(),
        &[ParameterType::Object(NativeClass::Dc)]
    );
    assert_eq!(update.return_type(), ReturnType::Void);
    let text = unit.to_ir_text();
    assert!(text.contains(".new-source "));
    assert!(text.contains(".constant "));
    assert!(text.contains(".call-native-discard "));
    let resumed = parse_unit_ir(&text).unwrap();
    assert_eq!(unit, resumed);
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    let options = ConsoleApplication {
        id: [49; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    let prg = build_console_application(&unit, &options).unwrap();
    assert_eq!(prg, build_console_application(&resumed, &options).unwrap());
    let records =
        ciq_prg::data::classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
            .unwrap();
    let fields = records
        .iter()
        .filter_map(|record| match record {
            ciq_prg::data::classes::Record::Class(class) => Some(&class.fields),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    for callback in [0x00800203, 0x00800204, 0x00800205] {
        let matches = fields
            .iter()
            .filter(|f| f.symbol_id() == callback)
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 1);
        assert!(matches[0].code_offset().is_some());
    }
}

#[test]
fn source_and_saved_ir_cannot_bypass_constructor_and_drawing_contracts() {
    for changed in [
        APP.replace("new HelloView()", "new Missing()"),
        APP.replace("new HelloView()", "new MinimalConsoleApp()"),
        APP.replace("new HelloView()", "new HelloView(1)"),
    ] {
        assert!(compile_sources(&[&changed, VIEW]).is_err(), "{changed}");
    }
    for changed in [
        VIEW.replace("View.initialize();", "AppBase.initialize();"),
        VIEW.replace("function initialize()", "function unused()"),
        VIEW.replace("onUpdate(dc)", "onUpdate(dc as Toybox.Lang.Number)"),
        VIEW.replace("Graphics.FONT_MEDIUM", "3"),
        VIEW.replace("Graphics.FONT_MEDIUM", "Graphics.FONT_UNKNOWN"),
        VIEW.replace("dc.clear();", "var invalid = dc.clear();"),
        VIEW.replace("dc.clear();", "self.onShow();"),
        VIEW.replace("dc.clear();", "var unsupported = [dc];"),
        VIEW.replace("dc.clear();", "var Graphics = 1; dc.clear();"),
    ] {
        assert!(compile_sources(&[APP, &changed]).is_err(), "{changed}");
    }
    let unit = compile_sources(&[APP, VIEW]).unwrap();
    let text = unit.to_ir_text();
    for changed in [
        text.replace("HelloView WatchUi.View span=", "Missing WatchUi.View span="),
        text.replace("WatchUi.View", "Application.AppBase"),
        text.replace("parameter-types=dc", "parameter-types=integer"),
        text.lines()
            .map(|line| {
                if line.starts_with(".constant ") {
                    format!(".integer {} 3", line.split_whitespace().nth(1).unwrap())
                } else {
                    line.into()
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
    ] {
        assert_ne!(changed, text);
        assert!(parse_unit_ir(&changed).is_err(), "{changed}");
    }
}
