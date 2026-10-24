use ciq_compiler::{compile_sources, parse_unit_ir};
mod common;

fn check(source: &str) -> ciq_compiler::CompilationUnit {
    let unit = compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}\n{e}"));
    common::pipeline(unit)
}

#[test]
fn records_preserve_optional_fields_annotations_and_mutation() {
    for source in [
        "using Toybox.Lang; typedef Opt as {:offset as Lang.Number,:endianness as Lang.Endian}; function make() as Opt {return {:offset=>0};} function f(b as Lang.ByteArray) as Lang.Numeric {var o=make();o[:offset]=1;return b.decodeNumber(Lang.NUMBER_FORMAT_UINT8,o);}",
        "function f(x as {:name as Toybox.Lang.String}) as Toybox.Lang.String {var name=x[:name];if(name!=null){return name;}return \"missing\";}",
        "function f() as {:name as Toybox.Lang.String,:count as Toybox.Lang.Number} {return {};}",
        "function f() as {:name as Toybox.Lang.String} {return {:name=>\"one\",:extra=>1};}",
        "function f() as Toybox.Lang.Number {var x={:count=>1,:label=>\"text\"};x[:count]=2;var n=x[:count];if(n!=null){return n;}return 0;}",
        "function f(x as {:count as Toybox.Lang.Number}) as Void {x.put(:count,2);x.remove(:count);}",
        "function f(x as {:child as {:count as Toybox.Lang.Number}}) as Toybox.Lang.Number {var c=x[:child];if(c!=null){var n=c[:count];if(n!=null){return n;}}return 0;}",
        "function f() as {:child as {:count as Toybox.Lang.Number}} {return {:child=>{:count=>1}};}",
        "class Box {var opts as {:count as Toybox.Lang.Number};function initialize(){self.opts={:count=>1};}function read() as Toybox.Lang.Number {var n=self.opts[:count];if(n!=null){return n;}return 0;}}",
        "function f(flag as Toybox.Lang.Boolean) as {:count as Toybox.Lang.Number} {return flag?{:count=>1}:{};}",
        "function f() as Toybox.Lang.Array<{:count as Toybox.Lang.Number}> {return [{:count=>1}];}",
        "class C {function value() as Toybox.Lang.Number{return 41;}} function f() as {:item as C} {return {:item=>new C()};} function g() as Toybox.Lang.Number {var o=f();var c=o[:item];if(c!=null){return c.value();}return 0;}",
    ] {
        check(source);
    }
}

#[test]
fn records_reject_forged_ir_and_preserve_api_and_constructor_effects() {
    use ciq_compiler::{
        api::{ApiContext, ApiLevel},
        compile_function_with_api, parse_function_ir,
    };
    let unit = check("function f() as {:count as Toybox.Lang.Number} {return {:count=>1};}");
    let text = unit.to_ir_text();
    let forged = text.replace("options(count=integer)", "options(count=string)");
    assert_ne!(text, forged);
    assert!(parse_unit_ir(&forged).is_err());
    for tag in [
        "options(x=void)",
        "options(x=integer;x=string)",
        "options(1x=integer)",
        "options(x=)",
    ] {
        let source =
            format!("ciq-ir\n.function f parameters=1 parameter-types={tag}\n.return-void\n.end\n");
        assert!(parse_function_ir(&source).is_err(), "accepted {tag}");
    }
    let source = "function f() as {:bytes as Toybox.Lang.ByteArray} {return {}; }";
    let old = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(2, 4, 0)).unwrap();
    assert!(compile_function_with_api(source, old).is_err());
    let source = "class Box {var n as Toybox.Lang.Number;function initialize(o as {:x as Toybox.Lang.Object}){o.toString();self.n=1;}}";
    assert!(compile_sources(&[source]).is_err());
    check(&source.replace("o.toString();self.n=1;", "self.n=1;o.toString();"));
}

#[test]
fn records_recheck_known_field_values_and_optional_reads() {
    for source in [
        "using Toybox.Lang; module Options {function wrong() as {:offset as Lang.String} {return {:offset=>\"wrong\"};}} function f(b as Lang.ByteArray) as Lang.Numeric {return b.decodeNumber(Lang.NUMBER_FORMAT_UINT8,Options.wrong());}",
        "function f() as {:n as Toybox.Lang.Number} {return {:n=>\"wrong\"};}",
        "function f(x as {:n as Toybox.Lang.Number}) as Toybox.Lang.Number {return x[:n];}",
        "function f(x as {:n as Toybox.Lang.Number}) as {:n as Toybox.Lang.String} {return x;}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "accepted {source}");
    }
}

#[test]
fn evolving_records_follow_references_and_control_flow() {
    for source in [
        r#"typedef Has as interface {var options as {:x as Toybox.Lang.Number};}; function f(owner as Has) as Toybox.Lang.Number or Toybox.Lang.String or Null {owner.options[:x]="changed";return owner.options[:x];}"#,
        r#"function f() as Toybox.Lang.String or Null {var o={:x=>1};o={:x=>"rebound"};return o[:x];}"#,
        r#"function f() as Toybox.Lang.Number or Toybox.Lang.String or Null {var o={:x=>1};var flag=true;var other={:x=>2};var selected=flag?o:other;selected[:x]="changed";return o[:x];}"#,
        r#"function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.String or Null {var o=flag?{:x=>1}:null;if(o!=null){o[:x]="changed";return o[:x];}return null;}"#,
        r#"function f() as Toybox.Lang.String or Null {var o={:x=>1};var saved=o[:x];o[:x]="changed";return o[:x];}"#,
        r#"function f(o as {:x as Toybox.Lang.Number}) as Toybox.Lang.String or Null {o[:x]="changed";return o[:x];}"#,
        r#"function f() as Toybox.Lang.String or Null {var o={:x=>1};var shared=o;shared[:x]="changed";return o[:x];}"#,
        r#"function f() as Toybox.Lang.String or Null {var o={:x=>1};o.put(:label,"added");return o[:label];}"#,
        r#"function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Number or Toybox.Lang.String or Null {var o={:x=>1};if(flag){o[:x]="changed";}return o[:x];}"#,
        r#"function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Number or Toybox.Lang.String or Null {var o={:x=>1};while(flag){o[:x]="changed";}return o[:x];}"#,
        r#"function f(key as Toybox.Lang.Symbol) as Toybox.Lang.Number or Toybox.Lang.String or Null {var o={:x=>1};o[key]="changed";return o[:x];}"#,
        r#"function f() as Toybox.Lang.Number or Null {var o={:x=>1};var shared=o;o={:x=>2};shared[:x]="old";return o[:x];}"#,
        r#"class Box {var o as {:x as Toybox.Lang.Number};function initialize(){self.o={:x=>1};}function f() as Toybox.Lang.String or Null {self.o[:x]="changed";return self.o[:x];}}"#,
        r#"function f() as Toybox.Lang.String or Null {var child={:x=>1};var o={:child=>child};child[:x]="changed";var c=o[:child];if(c!=null){return c[:x];}return null;}"#,
    ] {
        check(source);
    }
    for source in [
        r#"typedef Has as interface {var options as {:x as Toybox.Lang.Number};}; function f(owner as Has) as Toybox.Lang.Number or Null {owner.options[:x]="changed";return owner.options[:x];}"#,
        r#"function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Number or Null {var o={:x=>1};var other={:x=>2};var selected=flag?o:other;selected[:x]="changed";return o[:x];}"#,
        r#"function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Number or Null {var o=flag?{:x=>1}:null;return o[:x];}"#,
        r#"function f() as Toybox.Lang.Number or Null {var o={:x=>1};var shared=o;shared[:x]="changed";return o[:x];}"#,
        r#"function f(flag as Toybox.Lang.Boolean) as Toybox.Lang.Number or Null {var o={:x=>1};if(flag){o[:x]="changed";}return o[:x];}"#,
    ] {
        assert!(compile_sources(&[source]).is_err(), "accepted {source}");
    }
}

#[test]
fn saved_record_reads_cannot_claim_a_stale_field_contract() {
    let source = r#"function f() as Toybox.Lang.String or Null {var o={:x=>1};o={:x=>"rebound"};return o[:x];}"#;
    let unit = check(source);
    let text = unit.to_ir_text();
    let forged = text
        .lines()
        .map(|line| {
            if line.starts_with(".read-variable ") {
                line.replace("options(x=string)", "options(x=integer)")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(text.trim(), forged.trim());
    assert!(parse_unit_ir(&forged).is_err());
}

#[test]
fn record_example_preserves_the_complete_application_pipeline() {
    use ciq_compiler::{
        api::{ApiContext, ApiLevel},
        application::ConsoleApplication,
        compile_sources_with_api,
    };
    let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(3, 1, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[include_str!(
            "../../../examples/compiler-option-records/app.mc"
        )],
        context,
    )
    .unwrap();
    common::application(
        &unit,
        &ConsoleApplication {
            id: 0xd06b2552a0784ae99dd65e4b1c329ef1u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
}
