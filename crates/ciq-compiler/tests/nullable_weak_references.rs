use ciq_compiler::{
    ParameterType, ReturnType,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_function, compile_function_with_api, compile_sources, compile_sources_with_api,
    parse_function_ir, parse_function_ir_with_api, parse_unit_ir, parse_unit_ir_with_api,
};

#[test]
fn nullable_weak_values_preserve_signatures_joins_and_storage_proofs() {
    for source in [
        "function f(w as Toybox.Lang.WeakReference or Null) as Toybox.Lang.WeakReference or Null {return w;}",
        "function f(w as Toybox.Lang.WeakReference,b as Toybox.Lang.Boolean) as Toybox.Lang.WeakReference or Null {return b?w:null;}",
        "function f(w as Toybox.Lang.WeakReference,b as Toybox.Lang.Boolean) as Toybox.Lang.WeakReference or Null {return b?null:w;}",
        "function f(w as Toybox.Lang.WeakReference or Null,n as Toybox.Lang.WeakReference) as Void {var local=w;local=n;local=null;}",
        "function f(w as Toybox.Lang.WeakReference or Null) as Void {if(null==w){return;}var value=w.get();if(value!=null){value.toString();}}",
        "function f(w as Toybox.Lang.WeakReference or Null) as Toybox.Lang.Boolean {return w!=null && w.stillAlive();}",
        "function f(w as Toybox.Lang.WeakReference or Null) as Void {var local=w;while(local!=null){local.get();local=null;}}",
    ] {
        let f = compile_function(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
    }
    let f=compile_function("function f(w as Toybox.Lang.WeakReference or Null) as Toybox.Lang.WeakReference or Null {return w;}").unwrap();
    assert_eq!(f.parameter_types(), &[ParameterType::NullableWeakReference]);
    assert_eq!(f.return_type(), ReturnType::NullableWeakReference);
    assert!(
        parse_function_ir(
            &f.to_ir_text()
                .replace("result=nullable-weak-reference", "result=weak-reference")
        )
        .is_err()
    );
    for source in [
        "function f(w as Toybox.Lang.WeakReference or Null) as Void {w.get();}",
        "function f(w as Toybox.Lang.WeakReference or Null) as Toybox.Lang.Boolean {return w.stillAlive();}",
        "function f(w as Toybox.Lang.WeakReference or Null) as Toybox.Lang.WeakReference {return w;}",
        "function f(w as Toybox.Lang.WeakReference or Null) as Void {var local=w;if(local!=null){local=null;local.get();}}",
        "function f(w as Toybox.Lang.WeakReference or Null) as Void {var local=w;if(w!=null){local.get();}}",
        "function f(w as Toybox.Lang.WeakReference or Null) as Void {var present=w!=null;if(present){w.get();}}",
        "function f(w as Toybox.Lang.WeakReference or Null) as Void {if(w!=null && w.stillAlive()){w.get().toString();}}",
    ] {
        assert!(compile_function(source).is_err(), "accepted {source}");
    }
    let f = compile_function(
        "function f(w as Toybox.Lang.WeakReference or Null) as Void {if(w!=null){w.get();}}",
    )
    .unwrap();
    assert!(parse_function_ir(&f.to_ir_text().replace(".not-equal ", ".equal ")).is_err());
}

#[test]
fn weak_fields_and_calls_survive_saved_ir_and_null_metadata() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-weak-references/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".field-weak-reference fixed weak-reference\n"));
    assert!(text.contains(".field-weak-reference maybe nullable-weak-reference\n"));
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    let asm = unit.lower();
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    let options = ConsoleApplication {
        id: [0x54; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    let prg = build_console_application(&unit, &options).unwrap();
    assert_eq!(
        prg,
        build_console_application(&parse_unit_ir(&text).unwrap(), &options).unwrap()
    );
    let records =
        ciq_prg::data::classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
            .unwrap();
    let slots = records
        .iter()
        .flat_map(|record| match record {
            ciq_prg::data::classes::Record::Class(class) => class.fields.as_slice(),
            _ => &[],
        })
        .filter(|field| field.kind == 0 && field.symbol_id() < 0x0080_0000)
        .collect::<Vec<_>>();
    assert_eq!(slots.len(), 3);
    assert!(slots.iter().all(|field| field.value == 0));
    for edited in [
        text.replace(
            ".field-weak-reference fixed weak-reference",
            ".field-weak-reference fixed nullable-weak-reference",
        ),
        text.replace(
            ".field-weak-reference maybe nullable-weak-reference",
            ".field-weak-reference maybe weak-reference",
        ),
        text.replace(
            ".field-weak-reference maybe nullable-weak-reference",
            ".field-weak-reference maybe nullable-method",
        ),
        text.replace(
            "maybe result=nullable-weak-reference",
            "maybe result=weak-reference",
        ),
    ] {
        assert_ne!(text, edited);
        assert!(parse_unit_ir(&edited).is_err());
    }
    for source in [
        "class C {var w as Toybox.Lang.WeakReference;function initialize(){}}",
        "class C {var w as Toybox.Lang.WeakReference=null;function initialize(){}}",
        "class C {var w as Toybox.Lang.WeakReference;function initialize(n as Toybox.Lang.Number){self.w=n.weak();}}",
        "class C {var w as Toybox.Lang.WeakReference or Null=1;}",
        "class C {var w as Toybox.Lang.WeakReference;function initialize(w as Toybox.Lang.WeakReference or Null){self.w=w;}}",
        "class C {var w as Toybox.Lang.WeakReference or Null;function f() as Void {if(self.w!=null){self.w.get();}}}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "accepted {source}");
    }
}

#[test]
fn weak_nullability_keeps_api_requirements_and_unchecked_assertions() {
    let modern = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 2, 0)).unwrap();
    let initial = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap();
    let source = "function f(w as Toybox.Lang.WeakReference or Null) as Toybox.Lang.WeakReference or Null {return w;}";
    let f = compile_function_with_api(source, modern).unwrap();
    assert!(compile_function_with_api(source, initial).is_err());
    assert!(parse_function_ir_with_api(&f.to_ir_text(), initial).is_err());
    let source = "class C {var w as Toybox.Lang.WeakReference or Null;function f() as Void {}}";
    let unit = compile_sources_with_api(&[source], modern).unwrap();
    assert_eq!(
        unit,
        parse_unit_ir_with_api(&unit.to_ir_text(), modern).unwrap()
    );
    assert!(parse_unit_ir_with_api(&unit.to_ir_text(), initial).is_err());
    assert!(compile_sources_with_api(&[source], initial).is_err());
    let source = "function f(w as Toybox.Lang.Object or Null) as Toybox.Lang.WeakReference or Null {return w as Toybox.Lang.WeakReference or Null;}";
    let f = compile_function(source).unwrap();
    let identity = compile_function(
        "function f(w as Toybox.Lang.Object or Null) as Toybox.Lang.Object or Null {return w;}",
    )
    .unwrap();
    assert_eq!(f.lower(), identity.lower());
    assert_eq!(f, parse_function_ir(&f.to_ir_text()).unwrap());
}
