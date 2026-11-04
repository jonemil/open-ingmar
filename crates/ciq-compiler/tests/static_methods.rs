use ciq_compiler::{compile_sources, parse_function_ir, parse_unit_ir};
mod common;

#[test]
fn static_methods_keep_receiver_contracts_in_all_saved_stages() {
    let unit = compile_sources(&[include_str!(
        "../../../examples/compiler-static-methods/app.mc"
    )])
    .unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".function seed receiver=static"));
    assert!(text.contains(".static-self"));
    assert!(text.contains(".function __ciq_initialize_statics receiver=static"));
    assert!(
        unit.classes()
            .iter()
            .find(|c| c.name == "Counter")
            .unwrap()
            .methods
            .iter()
            .any(|f| f.name() == "add" && f.is_static_method())
    );
    let prg = common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0xb67c1691c9a5422a9d3202a0c016870cu128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    // Static methods must occupy statics with the SDK-observed method flags,
    // rather than merely having a body that happens to decode correctly.
    let symbols =
        ciq_prg::symbols::Symbols::parse(prg.unique_section(ciq_prg::SYMBOLS).unwrap().payload())
            .unwrap();
    let add = symbols.iter().find(|s| s.name == "add").unwrap().id;
    let records =
        ciq_prg::data::classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
            .unwrap();
    let flags = records
        .iter()
        .filter_map(|r| match r {
            ciq_prg::data::classes::Record::Class(c) => Some(c),
            _ => None,
        })
        .flat_map(|c| &c.fields)
        .filter(|f| f.kind == 6 && f.key >> 8 == add)
        .map(|f| f.key & 0xff)
        .collect::<Vec<_>>();
    assert_eq!(flags, vec![0x50]);
    unit.validate_api(
        ciq_compiler::api::ApiContext::new(
            ciq_compiler::api::ApiLevel(5, 2, 0),
            ciq_compiler::api::ApiLevel(3, 0, 0),
        )
        .unwrap(),
    )
    .unwrap();
    for source in [
        "class C{static function f(){return 1;}}function g(){var c=new C();return c.f();}",
        "class C{static function f(){return 1;}static function g(){return f();}}",
        "class C{static function f(){return self;}static var OWNER=self;}",
        "module M{var BASE=1;class C{static function f(){return BASE;}}}",
        "module M{function method(){return 1;}class C{static function f(){return method();}}}",
        "class C{static var N=1;static function f(){N+=1;self.N++;return N;}}",
        "using Toybox.Lang;class C{static function f(n as Lang.Number) as Lang.Number{return n;}}function take(bound as Method(n as Lang.Number) as Lang.Number) as Lang.Number{return bound.invoke(1);}function g(){var c=new C();return take(c.method(:f));}",
        "using Toybox.Lang;class C{static var N as Lang.Number=value();static function value() as Lang.Number{return 1;}}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}")));
    }
}

#[test]
fn static_bodies_cannot_claim_instance_state_or_native_callbacks() {
    for source in [
        "static function f(){}",
        "module M{static function f(){}}",
        "class C{var n=1;static function f(){return n;}}",
        "class C{var n=1;static function f(){self.n=2;}}",
        "class C{function value(){return 1;}static function f(){return value();}}function value(){return 2;}",
        "class C{function value(){return 1;}static function f(){return self.value();}}",
        "class C{static function initialize(){}}",
        "class C extends Toybox.WatchUi.View{static function onShow(){}}",
        "class C{static function f(){return self.hashCode();}}",
        "class C{static function f(){return method(:f);}}",
        "class C{static function f(n as Toybox.Lang.Number){}}function g(){var c=new C();c.f(\"bad\");}",
        "class C{static var S as Toybox.Lang.String=read();static function read() as Toybox.Lang.String{return S;}}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}

#[test]
fn saved_static_receivers_have_explicit_ownership_and_no_instance_proof() {
    let text = compile_sources(&["class C{static function owner(){return self;}}"])
        .unwrap()
        .to_ir_text();
    for forged in [
        text.replace("receiver=static", "receiver=instance"),
        text.replace(".static-self %0 C", ".static-self %0 Missing"),
        text.replace(".static-self %0 C", ".self %0 class=C"),
        text.replace(".static-self %0 C", ".module-self %0"),
        text.replace("receiver=static", "receiver=static receiver=instance"),
    ] {
        assert_ne!(text, forged);
        assert!(parse_unit_ir(&forged).is_err(), "{forged}");
    }
    let function = parse_function_ir("ciq-ir\n.function owner receiver=static result=module-value\n.static-self %0 C\n.return %0\n.end\n").unwrap();
    assert!(function.is_static_method());
    assert!(!function.is_instance_method());
    let typed = ciq_compiler::Function::static_method_with_signature(
        "owner".into(),
        vec![],
        ciq_compiler::ReturnType::ModuleValue,
        function.operations().to_vec(),
    )
    .unwrap();
    assert_eq!(function, typed);
    assert!(
        parse_unit_ir(&format!(
            "ciq-unit\n.owner global\n{}",
            function.to_ir_text()
        ))
        .is_err()
    );
}
