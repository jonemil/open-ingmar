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
    let source_field = left
        .to_ir_text()
        .replace("f(value=integer)", "f(value=source(Token@Lang.Object))");
    assert!(parse_function_ir(&source_field).is_ok());
    let source_parameter = left.to_ir_text().replace(
        "m(get()=integer)",
        "m(get(source(Token@Lang.Object))=integer)",
    );
    assert!(parse_function_ir(&source_parameter).is_ok());
    let unit_text = format!("ciq-unit\n.owner global\n{source_field}");
    assert!(parse_unit_ir(&unit_text).is_err());
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

#[test]
fn exact_source_members_preserve_identity_through_reads_calls_and_saved_ir() {
    let source = "class Token {function id() as Toybox.Lang.Number {return 41;}} class Other {} typedef Box as interface {var value as Token or Null; function echo(t as Token) as Token;}; class Holder {var value as Token or Null; function echo(t as Token) as Token {return t;}} function use(b as Box, t as Token) as Toybox.Lang.Number {b.value=t;var saved=b.value;var out=b.echo(t);return out.id();} function run(){return use(new Holder(),new Token());}";
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert!(
        text.contains("interface(f(value=nullable-source("),
        "{text}"
    );
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    assert!(unit.lower().to_text().contains("getsv"));
    for bad in [
        source.replace("b.value=t;", "b.value=new Other();"),
        source.replace("b.echo(t)", "b.echo(new Other())"),
        source.replace(
            "function echo(t as Token) as Token {return t;}",
            "function echo(t as Other) as Other {return t;}",
        ),
        source.replace(
            "var value as Token or Null; function echo(t as Token) as Token {",
            "var value as Other or Null; function echo(t as Token) as Token {",
        ),
    ] {
        assert!(compile_sources(&[&bad]).is_err(), "{bad}");
    }
}

#[test]
fn source_member_signatures_resolve_across_files_and_native_bases() {
    let declarations = r#"
        typedef Reader as interface {
            var value as Token or Null;
            function read() as Token;
        };
        function use(reader as Reader) as Token { return reader.read(); }
    "#;
    let implementation = r#"
        class Token extends Toybox.WatchUi.View {
            function initialize() { View.initialize(); }
        }
        class Holder {
            var value as Token or Null;
            function read() as Token { return new Token(); }
        }
        function run() { return use(new Holder()); }
    "#;
    let unit = compile_sources(&[declarations, implementation]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains("source(Token@WatchUi.View)"));
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    // Alter only the member contracts, retaining real class declarations/bases.
    let wrong_base = text.replace(
        "f(value=nullable-source(Token@WatchUi.View))",
        "f(value=nullable-source(Token@Lang.Object))",
    );
    assert!(parse_unit_ir(&wrong_base).is_err());
    let unknown = text.replace(
        "m(read()=source(Token@WatchUi.View))",
        "m(read()=source(Unknown@WatchUi.View))",
    );
    assert!(parse_unit_ir(&unknown).is_err());
    assert!(compile_sources(&[declarations]).is_err());
}

#[test]
fn nullable_source_fields_keep_snapshot_and_union_result_contracts() {
    let source = r#"
        class First { function number() as Toybox.Lang.Number { return 1; } }
        class Second { function number() as Toybox.Lang.Number { return 2; } }
        typedef Box as interface {
            var value as First or Null;
            function choose(first as Toybox.Lang.Boolean) as First or Second;
        };
        class Holder {
            var value as First or Null;
            function choose(first as Toybox.Lang.Boolean) as First or Second {
                return first ? new First() : new Second();
            }
        }
        function read(box as Box) as Toybox.Lang.Number {
            var value = box.value;
            box.value = null;
            if (value != null) { return value.number(); }
            return box.choose(false).number();
        }
        function run() { return read(new Holder()); }
    "#;
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let nullable_read = source.replace(
        "return box.choose(false).number();",
        "return box.value.number();",
    );
    assert!(compile_sources(&[&nullable_read]).is_err());
    let nullable_argument = r#"
        class Token {}
        function use(box as interface {function echo(t as Token) as Token;}, t as Token or Null) {
            return box.echo(t);
        }
    "#;
    assert!(compile_sources(&[nullable_argument]).is_err());
    let guarded = nullable_argument.replace(
        "return box.echo(t);",
        "if(t!=null){return box.echo(t);}return new Token();",
    );
    let unit = compile_sources(&[&guarded]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn saved_member_operations_reject_erased_or_wrong_source_arguments() {
    let source = r#"
        class Token {}
        class Other {}
        function use(box as interface {var value as Token or Null;function take(t as Token) as Void;},
                     token as Token, other as Other, erased as Toybox.Lang.Object) {
            box.value=token;
            box.take(token);
        }
    "#;
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    // Rewrite each operation independently with an extra read of another parameter.
    for parameter in [2, 3] {
        for prefix in [".member-write", ".call-native-discard"] {
            let rewritten = text
                .lines()
                .map(|line| {
                    if line.starts_with(prefix) {
                        let mut words = line.split_whitespace().collect::<Vec<_>>();
                        words[3] = "%999";
                        format!(".parameter %999 {parameter}\n{}", words.join(" "))
                    } else {
                        line.into()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            assert!(parse_unit_ir(&rewritten).is_err(), "{rewritten}");
        }
    }
}

#[test]
fn union_receivers_check_each_exact_signature_and_join_source_results() {
    let source = r#"
        class First { function number() as Toybox.Lang.Number {return 1;} }
        class Second { function number() as Toybox.Lang.Number {return 2;} }
        typedef Left as interface {function echo(x as First) as First;};
        typedef Right as interface {function echo(x as Second) as Second;};
        function use(box as Left or Right, arg) {return box.echo(arg).number();}
    "#;
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    let bad = source.replace("arg)", "arg as First)");
    assert!(compile_sources(&[&bad]).is_err());
    // A native Object alternative must not erase the source-required argument.
    let source = r#"
        class Token {}
        function use(value as Toybox.Lang.Object or interface {function equals(x as Token) as Toybox.Lang.Boolean;}, token as Toybox.Lang.Object) {
            return value.equals(token);
        }
    "#;
    assert!(compile_sources(&[source]).is_err());
}

#[test]
fn typed_interface_construction_rejects_void_fields_without_panicking() {
    let shape = ciq_compiler::StructuralInterface {
        declaration: None,
        fields: [("value".into(), ciq_compiler::ReturnType::Void)].into(),
        methods: Default::default(),
    };
    assert!(shape.validate().is_err());
}
