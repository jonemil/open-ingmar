use ciq_compiler::{compile_function, compile_sources, parse_function_ir, parse_unit_ir};

#[test]
fn callback_annotations_check_invocation_and_saved_signatures() {
    let source = "function use(f as Method(n as Toybox.Lang.Number) as Toybox.Lang.Number) as Toybox.Lang.Number {return f.invoke(41);}";
    let function = compile_function(source).unwrap();
    assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
    for bad in [
        source.replace("invoke(41)", "invoke(\"bad\")"),
        source.replace("invoke(41)", "invoke()"),
        source.replace("invoke(41)", "invoke(1,2)"),
    ] {
        assert!(compile_function(&bad).is_err(), "{bad}");
    }
    let source = "typedef Fn as Method(n as Toybox.Lang.Number) as Toybox.Lang.Number; function use(f as Fn or Null) as Toybox.Lang.Number {if(f!=null){return f.invoke(1);}return 0;}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn known_source_bindings_retain_signatures_and_reject_incompatible_assignments() {
    let source = r#"
        typedef Fn as Method(n as Toybox.Lang.Number) as Toybox.Lang.Number;
        class Worker {
            function step(n as Toybox.Lang.Number) as Toybox.Lang.Number {return n+1;}
            function bound() as Fn {return method(:step);}
        }
        function use(callback as Fn) as Toybox.Lang.Number {return callback.invoke(41);}
        function run() {var worker=new Worker();var callback=worker.method(:step);return use(callback);}
    "#;
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".bind-method"));
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    assert!(unit.lower().to_text().contains("__ciq_native_method"));
    let override_method = "ciq-ir\n.function method parameters=1 receiver=instance parameter-types=symbol result=method\n.parameter %0 0\n.symbol %1 toString\n.call-native %2 %0 method %1\n.return %2\n.end\n";
    let forged = text.replacen(".end-class", &format!("{override_method}.end-class"), 1);
    assert!(parse_unit_ir(&forged).is_err());
    let bad = source.replace(
        "step(n as Toybox.Lang.Number) as Toybox.Lang.Number {return n+1;}",
        "step(n as Toybox.Lang.String) as Toybox.Lang.String {return n;}",
    );
    assert!(compile_sources(&[&bad]).is_err());
    assert!(
        parse_unit_ir(&text.replace("callback(integer=>integer)", "callback(string=>string)"))
            .is_err()
    );
}

#[test]
fn callback_storage_and_containers_preserve_exact_signatures() {
    let source = r#"
        typedef Fn as Method(n as Toybox.Lang.Number) as Toybox.Lang.Number;
        class Worker {function step(n as Toybox.Lang.Number) as Toybox.Lang.Number{return n+1;}}
        class Store {var callback as Fn;function initialize(callback as Fn){self.callback=callback;}function run() as Toybox.Lang.Number {return self.callback.invoke(41);}}
        function array(callback as Fn) as Toybox.Lang.Array<Fn or Null> {return [callback,null];}
        function dict(callback as Fn) as Toybox.Lang.Dictionary<Fn,Fn or Null> {return {callback=>callback};}
        function maybe(callback as Fn) as Fn or Null {return callback;}
        function run() as Toybox.Lang.Number {
            var worker=new Worker();var callback=maybe(worker.method(:step));
            var saved=callback;callback=null;
            if(saved!=null){var values=array(saved);var value=values[0];if(value!=null){return new Store(value).run();}}
            return 0;
        }
    "#;
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let bad = source.replace("callback=null;", "callback=worker.method(:other);");
    // An unresolved selector deliberately keeps the existing erased lookup profile.
    assert!(compile_sources(&[&bad]).is_ok());
    let wrong = source.replace(
        "var callback as Fn;",
        "var callback as Method(n as Toybox.Lang.String) as Toybox.Lang.String;",
    );
    assert!(compile_sources(&[&wrong]).is_err());
}

#[test]
fn callback_results_keep_source_identities_and_void_invocation() {
    let source = r#"
        class Token {function number() as Toybox.Lang.Number{return 41;}}
        typedef Factory as Method() as Token;
        typedef Sink as Method(value as Token) as Void;
        function use(factory as Factory,sink as Sink) as Toybox.Lang.Number {var token=factory.invoke();sink.invoke(token);return token.number();}
    "#;
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let wrong = source.replace("sink.invoke(token);", "sink.invoke(1);");
    assert!(compile_sources(&[&wrong]).is_err());
    let wrong = source.replace(
        "var token=factory.invoke();",
        "var token=sink.invoke(factory.invoke());",
    );
    assert!(compile_sources(&[&wrong]).is_err());
}

#[test]
fn complete_callback_application_preserves_saved_stages() {
    let unit =
        compile_sources(&[include_str!("../../../examples/compiler-callbacks/app.mc")]).unwrap();
    let resumed = parse_unit_ir(&unit.to_ir_text()).unwrap();
    let app = ciq_compiler::application::ConsoleApplication {
        id: [1; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    assert_eq!(
        ciq_compiler::application::build_console_application(&unit, &app)
            .unwrap()
            .to_bytes(),
        ciq_compiler::application::build_console_application(&resumed, &app)
            .unwrap()
            .to_bytes()
    );
}

#[test]
fn native_binding_and_callback_assignments_match_exact_contracts() {
    let source = "function f() as Method() as Toybox.Lang.Number {return \"abc\".method(:length);}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    for (from, to) in [
        (
            "Method() as Toybox.Lang.Number",
            "Method(n as Toybox.Lang.Number) as Toybox.Lang.Number",
        ),
        (
            "Method() as Toybox.Lang.Number",
            "Method() as Toybox.Lang.String",
        ),
    ] {
        assert!(compile_sources(&[&source.replace(from, to)]).is_err());
    }
    let source = "typedef Fn as Method(n as Toybox.Lang.Number) as Toybox.Lang.Number; function f(callback as Method(n as Toybox.Lang.String) as Toybox.Lang.String) as Fn {return callback;}";
    assert!(compile_sources(&[source]).is_err());
    let dynamic = "typedef Fn as Method(n as Toybox.Lang.Number) as Toybox.Lang.Number; function f(callback as Toybox.Lang.Method) as Fn {return callback;}";
    let unit = compile_sources(&[dynamic]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn typed_callbacks_reject_nullable_calls_and_malformed_saved_signatures() {
    let source = "typedef Fn as Method(n as Toybox.Lang.Number) as Toybox.Lang.Number; function f(callback as Fn or Null) as Toybox.Lang.Number {return callback.invoke(1);}";
    assert!(compile_sources(&[source]).is_err());
    let function =
        compile_function("function f(callback as Method() as Void) {callback.invoke();}").unwrap();
    let text = function.to_ir_text();
    for bad in [
        text.replace("callback(=>void)", "callback(integer)"),
        text.replace("callback(=>void)", "callback(void=>void)"),
        text.replace("callback(=>void)", "callback(=>void=>void)"),
    ] {
        assert!(parse_function_ir(&bad).is_err(), "{bad}");
    }
    let mut ty = "Toybox.Lang.Number".to_owned();
    for _ in 0..100 {
        ty = format!("Method() as {ty}");
    }
    assert!(compile_function(&format!("function f(callback as {ty}) {{return 0;}}")).is_err());
}

#[test]
fn typed_methods_satisfy_matching_structural_member_contracts() {
    let source = "function call(value as interface {function invoke(n as Toybox.Lang.Number) as Toybox.Lang.Number;}) as Toybox.Lang.Number {return value.invoke(41);} function use(callback as Method(n as Toybox.Lang.Number) as Toybox.Lang.Number) as Toybox.Lang.Number {return call(callback);}";
    let api = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(1, 0, 0),
        ciq_compiler::api::ApiLevel(1, 0, 0),
    )
    .unwrap();
    let unit = ciq_compiler::compile_sources_with_api(&[source], api).unwrap();
    assert_eq!(
        unit,
        ciq_compiler::parse_unit_ir_with_api(&unit.to_ir_text(), api).unwrap()
    );
    let union = "typedef Invokable as interface {function invoke(n as Toybox.Lang.Number) as Toybox.Lang.Number;}; typedef Fn as Method(n as Toybox.Lang.Number) as Toybox.Lang.Number; function pass(value as Fn or Invokable) as Invokable {return value;}";
    let unit = compile_sources(&[union]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let wrong = source.replace(
        "use(callback as Method(n as Toybox.Lang.Number)",
        "use(callback as Method(n as Toybox.Lang.String)",
    );
    assert!(compile_sources(&[&wrong]).is_err());
}
