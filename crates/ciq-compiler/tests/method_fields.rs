#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{
    FieldKind,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_sources, compile_sources_with_api, parse_unit_ir,
};

#[test]
fn method_fields_preserve_saved_stages_and_null_metadata() {
    let unit =
        compile_sources(&[include_str!("../../../examples/compiler-methods/app.mc")]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".field-method fixed method\n"));
    assert!(text.contains(".field-method maybe nullable-method\n"));
    assert!(text.contains(".field-method startup nullable-method\n"));
    let resumed = parse_unit_ir(&text).unwrap();
    assert_eq!(unit, resumed);
    let store = unit
        .classes()
        .iter()
        .find(|class| class.name == "MethodStore")
        .unwrap();
    assert_eq!(store.fields[0].kind, FieldKind::Method { nullable: false });
    assert_eq!(store.fields[1].kind, FieldKind::Method { nullable: true });
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    let options = ConsoleApplication {
        id: [0x53; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    let prg = build_console_application(&unit, &options).unwrap();
    assert_eq!(prg, build_console_application(&resumed, &options).unwrap());
    let records =
        ciq_prg::data::classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
            .unwrap();
    let slots = records
        .iter()
        .flat_map(|record| match record {
            ciq_prg::data::classes::Record::Class(class) => class.fields.as_slice(),
            _ => &[],
        })
        // Source field IDs are allocated below the pinned native symbol range.
        .filter(|field| field.kind == 0 && field.symbol_id() < 0x0080_0000)
        .collect::<Vec<_>>();
    assert_eq!(slots.len(), 3, "{slots:?}");
    assert!(slots.iter().all(|field| field.value == 0));
    for edited in [
        text.replace(
            ".field-method fixed method",
            ".field-method fixed nullable-method",
        ),
        text.replace(
            ".field-method maybe nullable-method",
            ".field-method maybe method",
        ),
        text.replace(
            ".field-method maybe nullable-method",
            ".field-method maybe nullable-symbol",
        ),
        text.replace(
            ".field-method maybe nullable-method",
            ".field-method maybe nullable-method default=null",
        ),
    ] {
        assert_ne!(text, edited);
        assert!(parse_unit_ir(&edited).is_err());
    }
}

#[test]
fn method_fields_check_initialization_and_assignment_contracts() {
    for source in [
        "class C {var callback as Toybox.Lang.Method; function read() as Toybox.Lang.Method {return self.callback;}}",
        "class C {var callback as Toybox.Lang.Method; function initialize(){}}",
        "class C {var callback as Toybox.Lang.Method=null; function initialize(){}}",
        "class C {var callback as Toybox.Lang.Method or Null=1;}",
        "class C {var callback as Toybox.Lang.Method or Null=1.method(:abs);}",
        "class C {var callback as Toybox.Lang.Method; function initialize(){self.callback=self.method(:initialize);}}",
        "class C {var callback as Toybox.Lang.Method; function initialize(m as Toybox.Lang.Method){var old=self.callback;self.callback=m;}}",
        "class C {var callback as Toybox.Lang.Method; function initialize(m as Toybox.Lang.Method){self.callback=m;} function clear() as Void {self.callback=null;}}",
        "class C {var callback as Toybox.Lang.Method or Null; function set() as Void {self.callback=1;}}",
        "class C {var callback as Toybox.Lang.Method; function initialize(m as Toybox.Lang.Method or Null){self.callback=m;}}",
        "class C {var callback as Toybox.Lang.Method; function initialize(m as Toybox.Lang.Method,b as Toybox.Lang.Boolean){if(b){self.callback=m;}else{self.callback=m;}}}",
    ] {
        assert!(
            checking::compile_sources(&[source]).is_err(),
            "accepted {source}"
        );
    }
    for source in [
        "class C {var callback as Toybox.Lang.Method;function initialize(m as Toybox.Lang.Method){m.invoke();self.callback=m;}}",
        "module M {function call(m as Toybox.Lang.Method) as Void{m.invoke();}} class C {var callback as Toybox.Lang.Method;function initialize(m as Toybox.Lang.Method){M.call(m);self.callback=m;}}",
        "class C {var first as Toybox.Lang.Method;var second as Toybox.Lang.Method;function initialize(m as Toybox.Lang.Method){self.first=m;self.first.invoke();self.second=m;}}",
    ] {
        let error = compile_sources(&[source]).unwrap_err();
        assert!(error.diagnostic.message.contains("shutdown"), "{error:?}");
    }
    let source = "class C {var callback as Toybox.Lang.Method;function initialize(m as Toybox.Lang.Method){self.callback=m;m.invoke();}}";
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    let mut lines = text.lines().map(str::to_owned).collect::<Vec<_>>();
    let at = lines
        .iter()
        .position(|line| line.starts_with(".assign-self "))
        .unwrap();
    let assign = lines.remove(at);
    let at = lines
        .iter()
        .position(|line| line.starts_with(".return-void"))
        .unwrap();
    lines.insert(at, assign);
    assert!(parse_unit_ir(&lines.join("\n")).is_err());
    let source = "using Toybox.Lang as L; class C {var callback as L.Method;var maybe as L.Method or Null;function initialize(m as L.Method){self.callback=m;}function replace(m as L.Method) as Void{self.callback=m;}function read() as L.Method{return self.callback;}}";
    let initial = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap();
    let unit = compile_sources_with_api(&[source], initial).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn nullable_fields_require_snapshots_and_saved_ir_rechecks_reads() {
    let source = "class C {var callback as Toybox.Lang.Method or Null; function run() as Toybox.Lang.Object or Null {var saved=self.callback;self.callback=null;if(saved!=null){return saved.invoke();}return null;}}";
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    let edited = text.replace("callback result=nullable-method", "callback result=method");
    assert_ne!(text, edited);
    assert!(parse_unit_ir(&edited).is_err());
    for body in [
        "self.callback.invoke();",
        "if(self.callback!=null){self.callback.invoke();}",
        "var saved=self.callback;if(saved!=null){saved=null;saved.invoke();}",
    ] {
        let source = format!(
            "class C {{var callback as Toybox.Lang.Method or Null; function run() as Void {{{body}}}}}"
        );
        assert!(
            checking::compile_sources(&[&source]).is_err(),
            "accepted {source}"
        );
    }
}
