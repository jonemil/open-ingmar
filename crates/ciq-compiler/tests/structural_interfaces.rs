use ciq_compiler::{compile_function, compile_sources, parse_function_ir, parse_unit_ir};

#[test]
fn complete_application_binds_structural_fields_and_methods() {
    let unit =
        compile_sources(&[include_str!("../../../examples/compiler-interfaces/app.mc")]).unwrap();
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
            .to_bytes(),
    );
}

#[test]
fn named_and_anonymous_method_contracts_share_saved_ir_and_dispatch() {
    let sources = [
        "using Toybox.Lang; typedef Answer as interface {function answer() as Lang.Number;}; function read(x as Answer) as Lang.Number {return x.answer();} function text(x as interface {function toString() as Lang.String;}) as Lang.String {return x.toString();}",
        "class Box {function answer() as Toybox.Lang.Number {return 41;}} function start() {return read(new Box());}",
    ];
    let unit = compile_sources(&sources).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert!(unit.lower().to_text().contains("getsv"));
    let standalone=compile_function("function text(n as interface {function toString() as Toybox.Lang.String;}) as Toybox.Lang.String {return n.toString();}").unwrap();
    assert_eq!(
        standalone,
        parse_function_ir(&standalone.to_ir_text()).unwrap()
    );
}

#[test]
fn structural_assignments_require_actual_members_and_signatures() {
    let prefix = "typedef T as interface {function value(n as Toybox.Lang.Number) as Toybox.Lang.String;}; function use(x as T) as Toybox.Lang.String {return x.value(1);} ";
    for class in [
        "class C {}",
        "class C {function value() as Toybox.Lang.String {return \"x\";}}",
        "class C {function value(n as Toybox.Lang.String) as Toybox.Lang.String {return n;}}",
        "class C {function value(n as Toybox.Lang.Number) as Toybox.Lang.Number {return n;}}",
        "class C {function value(n as Toybox.Lang.Object) as Toybox.Lang.String {return \"x\";}}",
    ] {
        let source = format!("{prefix}{class} function start() {{return use(new C());}}");
        assert!(compile_sources(&[&source]).is_err(), "{class}");
    }
    let source = format!(
        "{prefix}class C {{function value(n as Toybox.Lang.Number) as Toybox.Lang.String {{return \"x\";}}}} function start() {{return use(new C());}}"
    );
    assert!(compile_sources(&[&source]).is_ok());
    let erased = format!("{prefix}function start(x as Toybox.Lang.Object) {{return use(x);}}");
    assert!(compile_sources(&[&erased]).is_err());
    let covariance = "typedef T as interface {function value() as Toybox.Lang.Object;}; class C {function value() as Toybox.Lang.Number {return 1;}} function use(x as T) {} function start() {use(new C());}";
    assert!(compile_sources(&[covariance]).is_err());
}

#[test]
fn interface_fields_are_invariant_witness_requirements() {
    let source = "typedef T as interface {var value as Toybox.Lang.Number; function answer() as Toybox.Lang.Number;}; class C {var value=1;function answer() as Toybox.Lang.Number {return 2;}} function pass(x as T) as T {return x;} function start() {return pass(new C());}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    for wrong in [
        source.replace("var value=1;", "var other=1;"),
        source.replace(
            "var value=1;",
            "var value as Toybox.Lang.Object;function initialize(){self.value=1;}",
        ),
    ] {
        assert!(compile_sources(&[&wrong]).is_err());
    }
}

#[test]
fn structural_fields_read_write_and_recheck_saved_operands() {
    let source = "typedef T as interface {var value as Toybox.Lang.Number;}; class C {var value=1;} function set(x as T, n as Toybox.Lang.Number) as Toybox.Lang.Number {x.value=n;return x.value;} function start(){return set(new C(),41);}";
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".member-write"));
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    assert!(unit.lower().to_text().contains("putv"));
    assert!(compile_sources(&[&source.replace("x.value=n;", "x.value=\"bad\";")]).is_err());
    assert!(parse_unit_ir(&text.replace(".member-read", ".unknown-read")).is_err());
    let branch = source.replace("x.value=n;", "x.value=n>0 ? 41 : 42;");
    let unit = compile_sources(&[&branch]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let compound = source.replace(
        "x.value=n;",
        "x.value += n>0 ? 1 : 2; x.value++; --x.value;",
    );
    let unit = compile_sources(&[&compound]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn nested_nullable_and_reordered_contracts_round_trip_without_erasure() {
    let source = "typedef Inner as interface {function number() as Toybox.Lang.Number;}; typedef Outer as interface {var child as Inner or Null;function apply(x as Inner) as Inner;}; function pass(x as Outer or Null) as Outer or Null {return x;}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let left=compile_function("function f(x as interface {var value as Toybox.Lang.Number;function get() as Toybox.Lang.Number;}) {return x;}").unwrap();
    let right=compile_function("function f(x as interface {function get() as Toybox.Lang.Number;var value as Toybox.Lang.Number;}) {return x;}").unwrap();
    assert_eq!(left.parameter_types(), right.parameter_types());
    let forged = left
        .to_ir_text()
        .replace("f(value=integer)", "f(value=source(Token@Lang.Object))");
    assert!(parse_function_ir(&forged).is_err());
    let forged = left.to_ir_text().replace(
        "m(get()=integer)",
        "m(get(source(Token@Lang.Object))=integer)",
    );
    assert!(parse_function_ir(&forged).is_err());
    let duplicate = left
        .to_ir_text()
        .replace("f(value=integer)", "f(value=integer);f(value=string)");
    assert!(parse_function_ir(&duplicate).is_err());
}

#[test]
fn source_witnesses_are_checked_on_returns_storage_and_member_arguments() {
    let prefix = "typedef T as interface {function answer() as Toybox.Lang.Number;}; class Good {function answer() as Toybox.Lang.Number {return 1;}} class Bad {} ";
    for body in [
        "function f() as T {return new Bad();}",
        "function f(x as T) {x=new Bad();}",
        "class Holder {var value as T or Null;function set() {self.value=new Bad();}}",
        "function use(x as interface {function take(t as T) as Void;}) {x.take(new Bad());}",
        "function use(x as interface {var value as T;}) {x.value=new Bad();}",
    ] {
        let source = format!("{prefix}{body}");
        assert!(compile_sources(&[&source]).is_err(), "{body}");
        assert!(
            compile_sources(&[&source.replace("new Bad()", "new Good()")]).is_ok(),
            "{body}"
        );
    }
    let nullable = "typedef T as interface {var value as Toybox.Lang.Number or Null;}; function f(x as T) as Toybox.Lang.Number {if(x.value != null){return x.value;}return 0;}";
    assert!(compile_sources(&[nullable]).is_err());
    let saved = nullable.replace(
        "if(x.value != null){return x.value;}",
        "var saved=x.value;if(saved != null){return saved;}",
    );
    assert!(compile_sources(&[&saved]).is_ok());
}

#[test]
fn structural_comparison_contracts_interoperate_with_core_interfaces() {
    let source = "typedef C as interface {function compare(a as Toybox.Lang.Object,b as Toybox.Lang.Object) as Toybox.Lang.Number;}; function convert(c as C) as Toybox.Lang.Comparator {return c;} function reverse(c as Toybox.Lang.Comparator) as C {return c;}";
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn interface_api_and_nesting_limits_are_rechecked() {
    let source = "function f(x as interface {function bytes() as Toybox.Lang.ByteArray;}) {}";
    let early = ciq_compiler::api::ApiContext::new(
        ciq_compiler::api::ApiLevel(2, 4, 0),
        ciq_compiler::api::ApiLevel(2, 4, 0),
    )
    .unwrap();
    assert!(ciq_compiler::compile_function_with_api(source, early).is_err());
    let function = compile_function(source).unwrap();
    assert!(ciq_compiler::parse_function_ir_with_api(&function.to_ir_text(), early).is_err());
    let deep = format!(
        "function f(x as {}Toybox.Lang.Number{}) {{}}",
        "interface {var value as ".repeat(70),
        ";}".repeat(70)
    );
    assert!(compile_function(&deep).is_err());
    let deep = format!(
        "ciq-ir\n.function f parameters=1 parameter-types={}integer{}\n.null %0\n.return %0\n.end\n",
        "interface(f(value=".repeat(70),
        "))".repeat(70)
    );
    assert!(parse_function_ir(&deep).is_err());
    for depth in [64, 65, 1000] {
        let source = format!("function f(x) {{return x{};}}", ".value".repeat(depth));
        assert!(compile_function(&source).is_err());
    }
}

#[test]
fn native_witnesses_require_the_actual_member_api_level() {
    use ciq_compiler::api::{ApiContext, ApiLevel};
    let early = ApiContext::new(ApiLevel(1, 2, 0), ApiLevel(1, 2, 0)).unwrap();
    let current = ApiContext::new(ApiLevel(1, 3, 0), ApiLevel(1, 3, 0)).unwrap();
    let body = "typedef Character as interface {function toChar() as Toybox.Lang.Char;}; function convert(n as Toybox.Lang.Number) as Character {return n;}";
    assert!(ciq_compiler::compile_function_with_api(body, early).is_err());
    let function = ciq_compiler::compile_function_with_api(body, current).unwrap();
    assert!(ciq_compiler::parse_function_ir_with_api(&function.to_ir_text(), early).is_err());
    let source = "typedef Character as interface {function toChar() as Toybox.Lang.Char;}; function use(n as Character) as Toybox.Lang.Char {return n.toChar();} function run() {return use(65);}";
    assert!(ciq_compiler::compile_sources_with_api(&[source], early).is_err());
    let unit = ciq_compiler::compile_sources_with_api(&[source], current).unwrap();
    assert!(unit.validate_api(early).is_err());
    assert!(ciq_compiler::parse_unit_ir_with_api(&unit.to_ir_text(), early).is_err());
    let member = "typedef Character as interface {function toChar() as Toybox.Lang.Char;}; function use(sink as interface {function accept(n as Character) as Void;}) {sink.accept(65);}";
    assert!(ciq_compiler::compile_function_with_api(member, early).is_err());
    assert!(ciq_compiler::compile_function_with_api(member, current).is_ok());
}
