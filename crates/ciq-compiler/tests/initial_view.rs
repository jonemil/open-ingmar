use ciq_compiler::{
    Function, Operation, ReturnType, TupleShape,
    application::{ConsoleApplication, build_console_application},
    compile_sources, parse_ir, parse_unit_ir,
};

#[test]
fn source_initial_view_keeps_a_positional_result_through_both_text_boundaries() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-initial-view/app.mc"
    )])
    .unwrap();
    let callback = unit.classes()[0]
        .methods
        .iter()
        .find(|f| f.name() == "getInitialView")
        .unwrap();
    assert_eq!(
        callback.return_type(),
        ReturnType::Tuple(TupleShape::OneView)
    );
    assert_eq!(callback.parameter_count(), 0);
    assert!(
        callback
            .operations()
            .iter()
            .any(|op| matches!(op.operation, Operation::TupleNew { .. }))
    );
    assert!(
        !callback
            .operations()
            .iter()
            .any(|op| matches!(op.operation, Operation::ArrayNew { .. }))
    );
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    assert_eq!(unit, resumed);
    let options = ConsoleApplication {
        id: [48; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        build_console_application(&unit, &options).unwrap(),
        build_console_application(&resumed, &options).unwrap()
    );
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    let prg = build_console_application(&unit, &options).unwrap();
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
        .filter(|field| field.symbol_id() == 0x008002a0)
        .collect::<Vec<_>>();
    assert_eq!(fields.len(), 1);
    let offset = fields[0].code_offset().unwrap() as usize;
    let code = ciq_mc::decode(prg.unique_section(ciq_prg::CODE).unwrap().payload()).unwrap();
    let index = code.iter().position(|op| op.offset == offset).unwrap();
    assert_eq!(code[index].instruction.opcode(), ciq_mc::Opcode::Argcincsp);
    assert_eq!(code[index + 1].instruction.opcode(), ciq_mc::Opcode::Getmv);
}

#[test]
fn callback_result_shape_cannot_be_forged_by_source_or_saved_ir() {
    for body in [
        "return [];",
        "return [new Toybox.WatchUi.View(), new Toybox.WatchUi.View()];",
        "return [1];",
        "return [null];",
        "return new Toybox.WatchUi.View();",
        "var a = [new Toybox.WatchUi.View()]; return a;",
        "return;",
    ] {
        let source = format!(
            "class C extends Application.AppBase {{ function start() {{}} function getInitialView() {{ {body} }} }}"
        );
        assert!(compile_sources(&[&source]).is_err(), "{source}");
    }
    for source in [
        "class C extends Application.AppBase { function getInitialView(n) { return [new Toybox.WatchUi.View()]; } }",
        "class C extends Application.AppBase { function getInitialView() as Toybox.Lang.Array<Toybox.WatchUi.View> { return [new Toybox.WatchUi.View()]; } }",
        "class C extends Application.AppBase { function getInitialView() { return [new Toybox.WatchUi.View()]; } function start() { self.getInitialView(); } }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let source = "class C extends Application.AppBase { function start() {} function getInitialView() { return [new Toybox.WatchUi.View()]; } }";
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".tuple-new "));
    let changed = text
        .lines()
        .map(|line| {
            if line.starts_with(".tuple-new ") {
                let fields: Vec<_> = line.split_whitespace().collect();
                format!(".array-new {} view", fields[1])
            } else {
                line.into()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(parse_unit_ir(&changed).is_err());
    for body in [
        ".parameter %0 0\n.tuple-new %1\n.return %1",
        ".parameter %0 0\n.parameter %1 0\n.tuple-new %2 %0 %1\n.return %2",
        ".integer %0 1\n.tuple-new %1 %0\n.return %1",
        ".new-native %0 Toybox.WatchUi.View\n.tuple-new %1 %0\n.return %1",
    ] {
        assert!(
            parse_ir(&format!(
                "ciq-ir\n.function f parameters=1 parameter-types=view\n{body}\n.end\n"
            ))
            .is_err()
        );
    }
    let f = parse_ir("ciq-ir\n.function f parameters=1 parameter-types=view\n.parameter %0 0\n.tuple-new %1 %0\n.return %1\n.end\n").unwrap();
    assert_eq!(f.return_type(), ReturnType::Tuple(TupleShape::OneView));
    assert_eq!(f, parse_ir(&f.to_ir_text()).unwrap());
    let mut ops = f.operations().to_vec();
    let last = ops.pop().unwrap();
    let result = if let Operation::Return { value } = last.operation {
        value
    } else {
        panic!()
    };
    ops.push(ciq_compiler::LocatedOperation {
        operation: Operation::NativeCall {
            result: ciq_compiler::ValueId(10),
            receiver: result,
            member: "add".into(),
            arguments: vec![],
        },
        span: None,
    });
    ops.push(ciq_compiler::LocatedOperation {
        operation: Operation::Return {
            value: ciq_compiler::ValueId(10),
        },
        span: None,
    });
    assert!(
        Function::with_signature(
            "bad".into(),
            f.parameter_types().into(),
            ReturnType::Tuple(TupleShape::OneView),
            ops
        )
        .is_err()
    );
}

#[test]
fn callback_shapes_are_checked_on_every_path_and_native_signature_levels() {
    let source = "class C extends Application.AppBase { function start() {} function getInitialView() { var n = 1; if (n == 1) { return [new Toybox.WatchUi.View()]; } return [new Toybox.WatchUi.View()]; } }";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let bad = source.replace("return [new Toybox.WatchUi.View()]; } }", "return []; } }");
    assert_ne!(bad, source);
    assert!(compile_sources(&[&bad]).is_err());
    let api = ciq_compiler::api::core();
    let result = &api
        .method("Toybox.Application.AppBase", "getInitialView")
        .unwrap()
        .method
        .result;
    assert!(
        matches!(result, ciq_compiler::api::Type::Union(shapes) if shapes.iter().all(|shape| matches!(shape, ciq_compiler::api::Type::Tuple(_))))
    );
    let initial = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(1, 0, 0),
        ciq_compiler::api::ApiLevel(1, 0, 0),
    )
    .unwrap();
    initial
        .check_type(api, &TupleShape::OneView.api_type())
        .unwrap();
    assert!(ciq_compiler::compile_sources_with_api(&[source], initial).is_err());
}
