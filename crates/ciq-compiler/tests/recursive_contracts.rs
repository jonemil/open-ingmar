use ciq_compiler::{compile_sources, parse_unit_ir};
#[test]
fn recursive_contracts_preserve_annotations_and_value_operations() {
    for source in [
        "using Toybox.Lang;typedef Tree as Lang.Array<Tree>;function child(t as Tree) as Tree{return t[0];}function size(t as Tree) as Lang.Number{return t[0].size();}",
        "using Toybox.Lang;typedef Recursive as interface {function next() as Recursive;function value() as Lang.Number;};function chain(x as Recursive) as Lang.Number {return x.next().value();}",
    ] {
        let unit = compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}"));
        let text = unit.to_ir_text();
        assert!(text.contains("rec("), "{text}");
        assert_eq!(
            unit,
            parse_unit_ir(&text).unwrap_or_else(|e| panic!("{text}: {e}"))
        );
        common::pipeline(unit.clone());
        assert_eq!(
            unit.lower(),
            ciq_assembly::Program::parse(&unit.lower().to_text()).unwrap()
        );
    }
}

#[test]
fn recursive_guards_mutual_aliases_and_closed_text_are_checked() {
    for source in [
        "using Toybox.Lang;typedef A as Lang.Array<B>;typedef B as Lang.Array<A>;function f(x as A) as B{return x[0];}",
        "using Toybox.Lang;typedef Tree as Lang.Array<Tree> or Null;function f(t as Tree) as Lang.Number{if(t==null){return 0;}return t.size();}",
        "using Toybox.Lang;typedef Map as Lang.Dictionary<Lang.String,Map>;function f(m as Map) as Map or Null{return m[\"child\"];}",
        "using Toybox.Lang;typedef Map as Lang.Dictionary<Lang.String,Map>;function f(m as Map,v as Map) as Void{m[\"child\"]=v;}",
    ] {
        let unit = compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}"));
        common::pipeline(unit);
    }
    for source in [
        "typedef A as A;function f(){}",
        "typedef A as B;typedef B as A;function f(){}",
        "typedef A as A or Null;function f(){}",
        "typedef A as {:next as A};typedef B as {:next as B,:z as Toybox.Lang.Number};function f(x as B) as A{return x;}",
        "using Toybox.Lang;typedef Tree as Lang.Array<Tree>;function f(t as Tree) as Lang.Number{return t[0];}",
        "using Toybox.Lang;typedef R as interface{function next() as R;function value() as Lang.Number;};function f(x as R) as Lang.String{return x.next().value();}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    for tag in [
        "ref(0)",
        "ref(18446744073709551615)",
        "rec(ref(0))",
        "rec(integer)",
    ] {
        let text = format!(
            "ciq-ir\n.function f parameters=1 parameter-types={tag}\n.null %0\n.return %0\n.end\n"
        );
        assert!(ciq_compiler::parse_function_ir(&text).is_err(), "{tag}");
    }
}

#[test]
fn recursive_nullability_witnesses_and_api_requirements_are_not_erased() {
    let source = "using Toybox.Lang;typedef R as interface{function next() as R;function touch() as Void;} or Null;function f(x as R) as Void {x.touch();}";
    assert!(compile_sources(&[source]).is_ok());
    let strict = ciq_compiler::CompileOptions {
        type_check: ciq_compiler::TypeCheckLevel::Strict,
        ..Default::default()
    };
    assert!(matches!(
        ciq_compiler::compile_sources_with_options(&[source], strict),
        Err(ciq_compiler::CompileError::TypeChecking(_))
    ));
    let guarded = source.replace("x.touch();", "if(x!=null){x.touch();}");
    common::pipeline(
        ciq_compiler::compile_sources_with_options(&[&guarded], strict)
            .unwrap()
            .artifact,
    );
    let api = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(5, 2, 0),
        ciq_compiler::api::ApiLevel(1, 0, 0),
    )
    .unwrap();
    let source = "using Toybox.Lang;typedef R as interface{function next() as R;function bytes() as Lang.ByteArray;};function f(){}";
    assert!(
        ciq_compiler::compile_sources_with_options(
            &[source],
            ciq_compiler::CompileOptions {
                api: Some(api),
                ..Default::default()
            }
        )
        .is_err()
    );
    for source in [
        "typedef R as interface{function next() as R;function missing() as Missing;};function f(){}",
        "typedef R as interface{function next() as R;function value() as Toybox.Lang.Number;};class Wrong {function next() as R{return new Wrong();}function value() as Toybox.Lang.String{return \"bad\";}} function f(x as R){}function g(){f(new Wrong());}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let source = "typedef A as interface{function next() as B;};typedef B as interface{function same() as B;function back() as A;};function f(x as A) as A{return x.next().same().back();}";
    common::pipeline(compile_sources(&[source]).unwrap());
}

#[test]
fn recursive_application_retains_source_witnesses_and_binary_stages() {
    let report = ciq_compiler::compile_sources_with_options(
        &[include_str!(
            "../../../examples/compiler-recursive-contracts/app.mc"
        )],
        ciq_compiler::CompileOptions {
            type_check: ciq_compiler::TypeCheckLevel::Strict,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(report.warnings.is_empty());
    let unit = report.artifact;
    let p = ciq_compiler::application::ConsoleApplication {
        id: 0x40e139c0f867462d85d9d4282ca9271bu128.to_be_bytes(),
        entry: "MinimalConsoleApp.onStart".into(),
    };
    common::application(&unit, &p);
}
mod common;

#[test]
fn appbase_references_do_not_authorize_construction() {
    use ciq_compiler::{ClassBase, api::Type};
    assert_eq!(ClassBase::ApplicationAppBase.constructed_type(), None);
    assert_eq!(
        ClassBase::ApplicationAppBase.reference_type(),
        Type::named("Toybox.Application.AppBase")
    );
    let source = r#"
        typedef Readable as interface {function value() as Toybox.Lang.Number;};
        class App extends Toybox.Application.AppBase {
            function initialize(){AppBase.initialize();}
            function value() as Toybox.Lang.Number{return 17;}
            function witness() as Readable{return self;}
            function same() as App{return self;}
        }
        function forward(x as App) as App{return x.same();}
        function nativeProjection(x as Toybox.Application.AppBase) as Toybox.Application.AppBase{return x;}
        function nullableNative(x as Toybox.Application.AppBase or Null)
                as Toybox.Application.AppBase or Null{return x;}
        function maybe(x as App or Null) as Toybox.Lang.Number {
            if(x==null){return -1;}return x.value();
        }
        function test(x as Toybox.Lang.Object) as Toybox.Lang.Boolean {
            return x instanceof App;
        }
        function asserted(x as Toybox.Lang.Object) as App or Null {
            return x as App or Null;
        }
    "#;
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains("class=App@Application.AppBase"), "{text}");
    assert!(text.contains("source(App@Application.AppBase)"), "{text}");
    common::pipeline(unit);
    assert!(parse_unit_ir(&text.replace("App@Application.AppBase", "App@Lang.Object")).is_err());
    for expression in ["new App()", "new Toybox.Application.AppBase()"] {
        assert!(
            compile_sources(&[
                source,
                &format!("function invalid(){{return {expression};}}")
            ])
            .is_err()
        );
    }
    let native = "ciq-ir\n.function f result=app-base\n.new-native %0 Toybox.Application.AppBase\n.return %0\n.end\n";
    assert!(ciq_compiler::parse_function_ir(native).is_err());
    let source = "ciq-ir\n.function f result=source(App@Application.AppBase)\n.new-source %0 App Application.AppBase\n.return %0\n.end\n";
    assert!(ciq_compiler::parse_function_ir(source).is_err());
}

#[test]
fn self_witnesses_are_anchored_to_the_enclosing_class() {
    let source = "typedef R as interface{function next() as R;function value() as Toybox.Lang.Number;};class Node{function next() as R{return self;}function value() as Toybox.Lang.Number{return 41;}}";
    let other = "class Other{function next() as R{return self;}function value() as Toybox.Lang.Number{return 41;}}";
    let unit = compile_sources(&[source, other]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains("class=Node@Lang.Object"), "{text}");
    common::pipeline(unit);
    for changed in [
        text.replace("class=Node@Lang.Object", "class=Other@Lang.Object"),
        text.replace("class=Node@Lang.Object", "class=Node@WatchUi.View"),
        text.replace(" class=Node@Lang.Object", ""),
    ] {
        assert!(parse_unit_ir(&changed).is_err(), "{changed}");
    }
    for bad in [
        "class Node{function next() as R{return self;}function value() as Toybox.Lang.String{return \"wrong\";}}",
        "class Other{}class Node{function next() as Other{return self;}}",
    ] {
        let declaration = source.split("class Node").next().unwrap();
        assert!(
            compile_sources(&[&format!("{declaration}{bad}")]).is_err(),
            "{bad}"
        );
    }
    for operation in [
        ".self %0 class=Node@Lang.Object class=Other@Lang.Object",
        ".self %0 type=object",
        ".self %0 class=Node@Unknown",
    ] {
        let text = format!(
            "ciq-ir\n.function f receiver=instance result=object\n{operation}\n.return %0\n.end\n"
        );
        assert!(ciq_compiler::parse_function_ir(&text).is_err(), "{text}");
    }
    let view = "class V extends Toybox.WatchUi.View{function initialize(){View.initialize();}function same() as V{return self;}}";
    common::pipeline(compile_sources(&[view]).unwrap());
}

#[test]
fn nested_dictionary_values_preserve_lookup_and_enumeration_contracts() {
    let source = r#"
        using Toybox.Lang;
        function first(d as Lang.Dictionary<Lang.String, Lang.Array<Lang.Number>>)
                as Lang.Number {
            var a = d.get("one");
            if (a == null) { return 0; }
            return a[0];
        }
        function values(d as Lang.Dictionary<Lang.String, Lang.Array<Lang.Number>>)
                as Lang.Number {
            return d.values()[0][0];
        }
    "#;
    common::pipeline(compile_sources(&[source]).unwrap());
}
