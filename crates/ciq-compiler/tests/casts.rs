use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    compile_function, compile_sources, compile_sources_with_api, parse_function_ir, parse_unit_ir,
};

#[test]
fn scalar_assertions_resume_without_runtime_checks_or_conversions() {
    for (class, tag) in [
        ("Number", "integer"),
        ("Long", "long"),
        ("Float", "float"),
        ("Double", "double"),
        ("Boolean", "boolean"),
        ("Char", "char"),
        ("String", "string"),
        ("Object", "object"),
        ("Method", "method"),
        ("Symbol", "symbol"),
        ("WeakReference", "weak-reference"),
        ("Dictionary", "dictionary"),
    ] {
        let source = format!(
            "function f(o as Toybox.Lang.Object) as Toybox.Lang.{class} {{ return o as Toybox.Lang.{class}; }}"
        );
        let function = compile_function(&source).unwrap();
        let text = function.to_ir_text();
        assert!(text.contains(&format!(" {tag} span=")), "{text}");
        assert_eq!(function, parse_function_ir(&text).unwrap());
        assert!(!function.lower().to_text().contains("isa"));
    }
    let uncast = compile_function("function f() { return 3; }").unwrap();
    let cast =
        compile_function("function f() { return 3 as Lang.String as Lang.Number; }").unwrap();
    assert_eq!(uncast.lower(), cast.lower());
    let source = "function f(o as Toybox.Lang.Object) { return 1 + o as Lang.Number; }";
    let function = compile_function(source).unwrap();
    assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
    assert!(
        compile_function("function f(o as Toybox.Lang.Object) { return -o as Lang.Number; }")
            .is_err()
    );
    assert!(
        compile_function("function f(o as Toybox.Lang.Object) { return -(o as Lang.Number); }")
            .is_ok()
    );
}

#[test]
fn alias_nullable_logical_and_conditional_casts_preserve_type_contracts() {
    for source in [
        "using Toybox.Lang as L; function f(o as Toybox.Lang.Object) as Toybox.Lang.String { return o as L.String; }",
        "function f(o as Toybox.Lang.Object) { return (o as Lang.String).length(); }",
        "function f(o as Toybox.Lang.Object or Null) as Toybox.Lang.String or Null { return o as Lang.String or Null; }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Boolean { return (o as Lang.Boolean) or false; }",
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Boolean { return o as Lang.Boolean or false; }",
        "function f(flag as Toybox.Lang.Boolean) { return (flag ? 1 : 2) as Lang.Number; }",
        "function f(o as Toybox.Lang.Object or Null) as Toybox.Lang.String { return o as Lang.String; }",
    ] {
        let function = compile_function(source).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
        function.lower();
    }
    let source = "module M { function length(s as Toybox.Lang.String) { return s.length(); } function f(o as Toybox.Lang.Object) { return M.length(o as Lang.String); } }";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn unsupported_targets_and_newer_api_types_are_rejected() {
    for target in ["Missing", "Lang.Void", "Null", "Lang.Array"] {
        let source =
            format!("function f(o as Toybox.Lang.Object) {{ var value = o as {target}; }}");
        assert!(compile_function(&source).is_err(), "{source}");
    }
    for tag in [
        "void",
        "null",
        "array-number",
        "source(C@Application.AppBase)",
        "unknown",
    ] {
        let text =
            format!("ciq-ir\n.function f\n.integer %0 1\n.cast %1 %0 {tag}\n.return %1\n.end\n");
        assert!(parse_function_ir(&text).is_err());
    }
    let source =
        "function f(o as Toybox.Lang.Object) as Toybox.Lang.Char { return o as Lang.Char; }";
    let low = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap();
    assert!(compile_sources_with_api(&[source], low).is_err());
    let high = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 3, 0)).unwrap();
    let unit = compile_sources_with_api(&[source], high).unwrap();
    assert!(unit.validate_api(low).is_err());
}

#[test]
fn cast_chains_preserve_the_supported_nesting_limit() {
    let casts = " as Lang.Number".repeat(64);
    compile_function(&format!("function f() {{ return 1{casts}; }}")).unwrap();
    let casts = " as Lang.Number".repeat(65);
    assert!(compile_function(&format!("function f() {{ return 1{casts}; }}")).is_err());
}

mod common;

#[test]
fn structural_assertions_reuse_value_types_and_preserve_saved_lowering() {
    for source in [
        "function f(x as Toybox.Lang.Object) as Toybox.Lang.Array<Toybox.Lang.Number> {return x as Toybox.Lang.Array<Toybox.Lang.Number>;}",
        "function f(x as Toybox.Lang.Object) as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.Number> {return x as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.Number>;}",
        "function f(x as Toybox.Lang.Object) as {:x as Toybox.Lang.Number} {return x as {:x as Toybox.Lang.Number};}",
        "function f(x as Toybox.Lang.Object) as [Toybox.Lang.Number,Toybox.Lang.String] {return x as [Toybox.Lang.Number,Toybox.Lang.String];}",
        "function f(x as Toybox.Lang.Object) as Toybox.Lang.Number or Toybox.Lang.String {return x as Toybox.Lang.Number or Toybox.Lang.String;}",
        "function f(x as Toybox.Lang.Object) as Toybox.Lang.Comparable {return x as Toybox.Lang.Comparable;}",
        "function f(x as Toybox.Lang.Object) as interface {function size() as Toybox.Lang.Number;} {return x as interface {function size() as Toybox.Lang.Number;};}",
        "function f(x as Toybox.Lang.Object) as Method(n as Toybox.Lang.Number) as Toybox.Lang.Number {return x as Method(n as Toybox.Lang.Number) as Toybox.Lang.Number;}",
        "class C {function n() as Toybox.Lang.Number {return 41;}} function f(x as Toybox.Lang.Object) as Toybox.Lang.Array<C> {return x as Toybox.Lang.Array<C>;}",
        "class C {function n() as Toybox.Lang.Number {return 41;}} class D {function n() as Toybox.Lang.Number {return 42;}} function f(x as Toybox.Lang.Object) as C or D {return x as C or D;}",
        "class C {function n() as Toybox.Lang.Number {return 41;}} function f(x as Toybox.Lang.Object) as C or Toybox.Lang.Number {return x as C or Toybox.Lang.Number;}",
        "typedef Options as {:x as Toybox.Lang.Number}; function f(x as Toybox.Lang.Object or Null) as Options or Null {return x as Options or Null;}",
        "function f(x as Toybox.Lang.Object,flag as Toybox.Lang.Boolean) as Toybox.Lang.Boolean {return (x as Toybox.Lang.Boolean) or flag;}",
    ] {
        let unit = compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}\n{e}"));
        let unit = common::pipeline(unit);
        assert!(!unit.lower().to_text().contains("isa"));
    }
}

#[test]
fn record_assertions_preserve_identity_and_distinct_static_views() {
    for source in [
        r#"function f() as Toybox.Lang.Number or Toybox.Lang.String or Null {var child={:x=>1};var holders=[child];var viewed=holders as Toybox.Lang.Array<{:x as Toybox.Lang.Number}>;viewed[0][:x]="changed";return child[:x];}"#,
        r#"function f() as Toybox.Lang.Number or Null {var original={:x=>1};var view=original as {:x as Toybox.Lang.String};return original[:x];}"#,
        r#"function f() as Toybox.Lang.String or Null {var original={:x=>1};var view=original as {:x as Toybox.Lang.String};return view[:x];}"#,
        r#"function f() as Toybox.Lang.Number or Toybox.Lang.String or Null {var original={:x=>1};var erased=original as Toybox.Lang.Object;var view=erased as {:x as Toybox.Lang.Number};view[:x]="changed";return original[:x];}"#,
        r#"function f() as Toybox.Lang.Number or Toybox.Lang.String or Null {var child={:x=>1};var original={:child=>child};var view=original as {:child as {:x as Toybox.Lang.Number}};var c=view[:child];if(c!=null){c[:x]="changed";}return child[:x];}"#,
    ] {
        common::pipeline(compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}\n{e}")));
    }
    let stale = r#"function f() as Toybox.Lang.Number or Null {var child={:x=>1};var holders=[child];var viewed=holders as Toybox.Lang.Array<{:x as Toybox.Lang.Number}>;viewed[0][:x]="changed";return child[:x];}"#;
    assert!(compile_sources(&[stale]).is_err());
    let nested = r#"function f() as Toybox.Lang.Number or Null {var child={:x=>1};var outer={:child=>child};var holders=[outer];var viewed=holders as Toybox.Lang.Array<{:child as {:x as Toybox.Lang.Number}}>;var key=:child;viewed[0][key]="changed";var c=outer[:child];if(c!=null){return c[:x];}return 0;}"#;
    assert!(compile_sources(&[nested]).is_err());
}

#[test]
fn asserted_contracts_do_not_bypass_api_or_source_declaration_checks() {
    let old = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(4, 2, 0)).unwrap();
    assert!(compile_sources_with_api(&["function f(x as Toybox.Lang.Object) as Toybox.Lang.Comparator {return x as Toybox.Lang.Comparator;}"],old).is_err());
    let unit=compile_sources(&["class C {} function f(x as Toybox.Lang.Object) as Toybox.Lang.Array<C> {return x as Toybox.Lang.Array<C>;}"]).unwrap();
    let forged = unit
        .to_ir_text()
        .replace("source(C@Lang.Object)", "source(Missing@Lang.Object)");
    assert_ne!(unit.to_ir_text(), forged);
    assert!(parse_unit_ir(&forged).is_err());
    for target in ["Void", "Null", "{:x as Void}"] {
        let source = format!("function f(x as Toybox.Lang.Object){{var value=x as {target};}}");
        assert!(compile_sources(&[&source]).is_err(), "{source}");
    }
}

#[test]
fn cast_example_checks_complete_prg_generation_before_the_audit() {
    use ciq_compiler::application::ConsoleApplication;
    let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(5, 0, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[include_str!("../../../examples/compiler-casts/app.mc")],
        context,
    )
    .unwrap();
    common::application(
        &unit,
        &ConsoleApplication {
            id: 0x91645fbd41764dba942e4de3adf060edu128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
}

#[test]
fn typed_single_source_assertions_normalize_to_the_existing_source_operation() {
    use ciq_compiler::{
        Function, LocatedOperation, Operation, SourceClass, SourceUnion, ValueId, api::Type,
    };
    let class = SourceClass {
        name: "C".into(),
        base: ciq_compiler::ClassBase::LangObject,
    };
    let target = Type::SourceValue(Box::new(SourceUnion::new(vec![class], false).unwrap()));
    let function = Function::new(
        "f".into(),
        vec![
            LocatedOperation {
                operation: Operation::Integer {
                    result: ValueId(0),
                    value: 1,
                },
                span: None,
            },
            LocatedOperation {
                operation: Operation::Cast {
                    result: ValueId(1),
                    operand: ValueId(0),
                    target,
                },
                span: None,
            },
            LocatedOperation {
                operation: Operation::Return { value: ValueId(1) },
                span: None,
            },
        ],
    )
    .unwrap();
    assert!(matches!(
        function.operations()[1].operation,
        Operation::SourceCast { .. }
    ));
    assert_eq!(function, parse_function_ir(&function.to_ir_text()).unwrap());
}
