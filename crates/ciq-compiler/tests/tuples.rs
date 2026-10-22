use ciq_compiler::{compile_sources, parse_unit_ir};

fn round_trip(source: &str) -> ciq_compiler::CompilationUnit {
    let unit = compile_sources(&[source]).unwrap();
    let saved = unit.to_ir_text();
    assert!(
        unit == parse_unit_ir(&saved).unwrap(),
        "unit changed after saved IR: {saved}"
    );
    let assembly = unit.lower();
    assert_eq!(
        assembly,
        ciq_assembly::Program::parse(&assembly.to_text()).unwrap()
    );
    unit
}

#[test]
fn positional_construction_calls_reads_and_nullable_guards_resume_from_ir() {
    round_trip(
        r#"
        using Toybox.Lang;
        typedef Pair as [Lang.Number, Lang.String];
        function make(n as Lang.Number) as Pair { return [n, "tuple"]; }
        function first(x as Pair) as Lang.Number { return x[0]; }
        function second(x as Pair) as Lang.String { return x[1]; }
        function select(x as Pair, i as Lang.Number) as Lang.Number or Lang.String { return x[i]; }
        function nullable(x as Pair or Null) as Lang.Number { if (x != null) { return x[0]; } return 0; }
        function nested() as [Lang.Number, [Lang.String]] { return [1, ["nested"]]; }
        function nestedRead() as Lang.String { var x = nested(); return x[1][0]; }
        function sized() as Lang.Number { var x = make(3); return x.size(); }
        function consume() as Lang.Number { return first([42, "answer"]); }
        function widened(x as [Lang.Number]) as [Lang.Numeric] { return x; }
        function array(x as [Lang.Number]) as Lang.Array<Lang.Number> { return x; }
        function view(x as [Toybox.WatchUi.View]) as [Toybox.WatchUi.View] { return x; }
        function either(x as [Lang.Number, Lang.String] or [Lang.Number, Lang.Boolean]) as Lang.Number { return x[0]; }
    "#,
    );
}

#[test]
fn exact_source_slots_survive_parameters_results_and_indexing() {
    round_trip(
        r#"
        using Toybox.Lang;
        class Token { function value() as Lang.Number { return 42; } }
        class Other { function value() as Lang.Number { return 7; } }
        function make() as [Token, Lang.String] { return [new Token(), "token"]; }
        function echo(x as [Token, Lang.String]) as [Token, Lang.String] { return x; }
        function test() as Lang.Number { var x = echo(make()); return x[0].value(); }
        function choose(x as [Token, Other], i as Lang.Number) as Token or Other { return x[i]; }
        function unionSlot(x as [Token] or [Other]) as Token or Other { return x[0]; }
        function nestedSource(x as [Lang.Number, [Token]]) as Token { return x[1][0]; }
    "#,
    );
}

#[test]
fn tuple_fields_callbacks_and_conditional_construction_keep_their_contracts() {
    round_trip(
        r#"
        using Toybox.Lang;
        typedef Pair as [Lang.Number, Lang.String];
        typedef Factory as Method() as Pair;
        class Token { function value() as Lang.Number { return 42; } }
        class Box {
            var pair as [Token, Lang.String];
            var optional as Pair or Null;
            function initialize() { self.pair = [new Token(), "field"]; }
            function read() as Lang.Number { return self.pair[0].value(); }
            function set() as Void { self.optional = [7, "optional"]; }
        }
        class Worker { function pair() as Pair { return [41, "callback"]; } }
        function callback(factory as Factory) as Pair { return factory.invoke(); }
        function choose(flag as Lang.Boolean) as Pair { return flag ? [1, "yes"] : [2, "no"]; }
    "#,
    );
}

#[test]
fn mixed_literal_and_nested_inference_uses_the_shared_tuple_model() {
    round_trip(
        r#"
        using Toybox.Lang;
        function pair() as Lang.String { var x = [42, "answer"]; return x[1]; }
        function scalarWidths() as Lang.Long { var x = [1, 2l, 3.0f, 4.0d]; return x[1]; }
        function flags() as Lang.Boolean { var x = [true, 'a', "text"]; return x[0]; }
        function nullable() as Lang.String { var x = [null, "value"]; return x[1]; }
        function justNull() as Null { var x = [null]; return x[0]; }
        function nested() as Lang.Number { var x = [[1, 2], ["nested"]]; return x[0][1]; }
        function innerTuple() as Lang.String { var x = [1, [2, "nested"]]; return x[1][1]; }
        function branch(flag as Lang.Boolean) as Lang.Boolean {
            var x = [7, [flag ? true : false, true]];
            return x[1][0];
        }
        function scalar(n as Lang.Number) as Lang.Number { var x = [n, n + 1]; x.add(3); return x[0]; }
        function contextual() as Lang.Array<Lang.Object or Null> { return [1, "array", null]; }
        function numeric() as Lang.Array<Lang.Numeric> { return [1, 2l, 3.0f, 4.0d]; }
        function format() as Lang.String { var x = [42, "answer"]; return Lang.format("$1$ $2$", x); }
        class Token { function number() as Lang.Number { return 41; } }
        function source() as Lang.Number { var x = [new Token(), ["source"]]; return x[0].number(); }
    "#,
    );
}

#[test]
fn tuple_lengths_and_indexes_use_full_number_encoding() {
    let slots = vec!["Toybox.Lang.Number"; 130].join(",");
    let elements = vec!["1"; 130].join(",");
    let unit = round_trip(&format!(
        "function f() as [{slots}] {{ return [{elements}]; }}"
    ));
    let text = unit.lower().to_text();
    assert!(text.contains("ipush2"), "{text}");
}

#[test]
fn saved_ir_rechecks_slot_and_nested_source_claims() {
    let unit = round_trip("class A {} class B {} function f() as [A] { return [new A()]; }");
    let text = unit.to_ir_text();
    let forged = text.replace(
        "result=tuple(source(A@Lang.Object))",
        "result=tuple(source(B@Lang.Object))",
    );
    assert_ne!(text, forged);
    assert!(parse_unit_ir(&forged).is_err());
    for tag in [
        "tuple()",
        "tuple(void)",
        "tuple(tuple())",
        "one-view",
        "view-delegate",
    ] {
        let text = format!(
            "ciq-unit\n.owner global\nciq-ir\n.function f parameters=1 parameter-types={tag}\n.parameter %0 0\n.return %0\n.end\n"
        );
        assert!(parse_unit_ir(&text).is_err(), "accepted {tag}");
    }
    assert!(
        ciq_compiler::TupleType::new(vec![ciq_compiler::ReturnType::Union(
            ciq_compiler::api::Type::Void
        )])
        .is_err()
    );
}

#[test]
fn malformed_shapes_indexes_and_unproven_slot_identities_are_rejected() {
    for source in [
        "function f() as [Toybox.Lang.Number] { return [\"wrong\"]; }",
        "function f() as [Toybox.Lang.Number] { return [1, 2]; }",
        "function f(x as [Toybox.Lang.Number]) { return x[-1]; }",
        "function f(x as [Toybox.Lang.Number]) { return x[1]; }",
        "function f(x as [Toybox.Lang.Number]) { return x[\"wrong\"]; }",
        "function f(x as [Toybox.Lang.Number, Toybox.Lang.String], i as Toybox.Lang.Number) as Toybox.Lang.Number { return x[i]; }",
        "class A {} class B {} function f() as [A] { return [new B()]; }",
        "class A {} function f(x as [Toybox.Lang.Object]) as [A] { return x; }",
        "function f(x as []) { return x; }",
        "function f(x as [Toybox.Graphics.Dc]) { return x; }",
        "function f(x as [Toybox.Lang.Number]) { x[0] = \"changed\"; }",
        "function f(x as [Toybox.Lang.Number]) { x.add(2); }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "accepted {source}");
    }
}
