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
        "tuple(;)",
        "tuple(void)",
        "tuple(tuple(void))",
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
    ] {
        assert!(compile_sources(&[source]).is_err(), "accepted {source}");
    }
}

#[test]
fn tuple_slot_replacement_updates_locals_parameters_branches_and_source_identity() {
    round_trip(
        r#"
        using Toybox.Lang;
        class Token { function number() as Lang.Number { return 41; } }
        function parameter(x as [Lang.Number, Lang.String]) as [Lang.String, Lang.String] {
            x[0] = "changed"; return x;
        }
        function local() as Lang.String { var x = [1, "old"]; x[0] = "changed"; return x[0]; }
        function source() as Lang.Number { var x = [1, "old"]; x[0] = new Token(); return x[0].number(); }
        function nested() as Lang.String { var x = [1, "old"]; x[0] = [2, "nested"]; return x[0][1]; }
        function branch(flag as Lang.Boolean) as Lang.Number or Lang.String {
            var x = [1, "old"]; if (flag) { x[0] = "changed"; } return x[0];
        }
        function alternatives(x as [Lang.Number, Lang.String] or [Lang.Boolean, Lang.Number]) as Lang.String {
            x[0] = "changed"; return x[0];
        }
        function compound(x as [Lang.Number, Lang.String]) as Lang.Number { x[0] += 2; x[0]++; return x[0]; }
        function effects(x as [Lang.Number, Lang.String], flag as Lang.Boolean) as Lang.Number {
            x[0] = flag ? 41 : 42; return x[0];
        }
        function aliasView() as Lang.Number { var x = [1, "old"]; var y = x; y[0] = "changed"; return x[0]; }
    "#,
    );
}

#[test]
fn tuple_writes_reject_stale_results_and_invalid_indices() {
    for source in [
        r#"function f(x as [Toybox.Lang.Number]) as [Toybox.Lang.Number] { x[0] = "changed"; return x; }"#,
        r#"function f(x as [Toybox.Lang.Number]) { x[1] = 2; }"#,
        r#"function f(x as [Toybox.Lang.Number]) { x["bad"] = 2; }"#,
    ] {
        assert!(compile_sources(&[source]).is_err(), "accepted {source}");
    }
    let unit = round_trip(
        r#"function f(x as [Toybox.Lang.Number]) as [Toybox.Lang.String] { x[0] = "changed"; return x; }"#,
    );
    let text = unit.to_ir_text();
    assert!(text.contains(".tuple-write"));
    assert!(parse_unit_ir(&text.replace("result=tuple(string)", "result=tuple(integer)")).is_err());
}

#[test]
fn read_only_tuple_parameters_preserve_the_full_parameter_frame_budget() {
    let parameters = (0..254)
        .map(|i| format!("p{i} as [Toybox.Lang.Number]"))
        .collect::<Vec<_>>()
        .join(",");
    round_trip(&format!(
        "function f({parameters}) as Toybox.Lang.Number {{ return p0[0]; }}"
    ));
    round_trip(&format!(
        "function f({parameters}) as Toybox.Lang.Number {{ p0[0] = 42; return p0[0]; }}"
    ));
    assert!(compile_sources(&[&format!("function f({parameters}) as Toybox.Lang.Number {{ p0[0] = 42; p1[0] = 43; return p0[0]; }}")]).is_err());
}

#[test]
fn tuple_appends_keep_cardinality_source_identity_and_saved_stage_contracts() {
    round_trip(
        r#"
        using Toybox.Lang;
        class Token { function number() as Lang.Number { return 41; } }
        function addResult(x as [Lang.Number]) as [Lang.Number, Lang.String] { return x.add("tail"); }
        function allResult(x as [Lang.Number], y as [Lang.String, Lang.Boolean]) as [Lang.Number, Lang.String, Lang.Boolean] { return x.addAll(y); }
        function literal(x as [Lang.Number]) as [Lang.Number, Lang.Number, Lang.Number] { x.addAll([2,3]); return x; }
        function empty(x as [Lang.Number]) as [Lang.Number] { x.addAll([]); return x; }
        function local() as Lang.String { var x=[1,"old"]; x.add("tail"); return x[2]; }
        function source() as Lang.Number { var x=[1,"old"]; x.add(new Token()); return x[2].number(); }
        function allSource(x as [Lang.Number], tail as [Token]) as Lang.Number { x.addAll(tail); return x[1].number(); }
        function returnedAlias(x as [Lang.Number]) as Lang.String { var y=x.add("tail"); return y[1]; }
        function updatedReceiver(x as [Lang.Number]) as Lang.String { var y=x.add("tail"); return x[1]; }
        function nested(x as [Lang.Number]) as [Lang.Number, Lang.String, Lang.Number] { x.add(x.add("tail").size()); return x; }
        function expression(x as [Lang.Number]) as Lang.Number { return x.add("tail").size()+x.size(); }
        function branching(x as [Lang.Number], flag as Lang.Boolean) as Lang.Number { if (flag) { x.add("tail"); } return x.size(); }
        function afterBranch(x as [Lang.Number], flag as Lang.Boolean) as Lang.Boolean { if (flag) { x.add("tail"); } x.add(true); return x[0] == 1; }
        function conditionalTail(x as [Lang.Number], flag as Lang.Boolean) as Lang.Number { x.addAll(flag ? [2,"text"] : [3,false]); return x.size(); }
        function addContainer(x as [Lang.Number]) as Lang.String { x.add([2,"nested"]); return x[1][1]; }
        function addEmpty(x as [Lang.Number]) as Lang.Number { x.add([]); return x[1].size(); }
        function callerAlias() as Lang.Number { var x=[1,"old"]; var y=x; y.add(2); return x[0]; }
    "#,
    );
}

#[test]
fn tuple_append_rejects_unknown_shapes_wrong_arity_stale_results_and_loops() {
    for source in [
        r#"function f(x as [Toybox.Lang.Number]) as [Toybox.Lang.Number] { x.add("tail"); return x; }"#,
        r#"function f(x as [Toybox.Lang.Number]) { x.add(); }"#,
        r#"function f(x as [Toybox.Lang.Number]) { x.addAll(1); }"#,
    ] {
        assert!(compile_sources(&[source]).is_err(), "accepted {source}");
    }
    let unit = round_trip(
        r#"function f(x as [Toybox.Lang.Number]) as [Toybox.Lang.Number,Toybox.Lang.String] { return x.add("tail"); }"#,
    );
    let text = unit.to_ir_text();
    assert!(text.contains(".tuple-add "));
    let forged = text.replace(
        "result=tuple(integer;string)",
        "result=tuple(integer;integer)",
    );
    assert_ne!(text, forged);
    assert!(parse_unit_ir(&forged).is_err());
}

#[test]
fn tuple_append_and_empty_tail_recheck_api_availability_after_reload() {
    use ciq_compiler::{
        api::{ApiContext, ApiLevel},
        compile_sources_with_api, parse_unit_ir_with_api,
    };
    let initial = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let available = ApiContext::new(ApiLevel(1, 3, 0), ApiLevel(1, 3, 0)).unwrap();
    for body in ["return x.add(2);", "return x.addAll([]);"] {
        let source = format!("function f(x as [Toybox.Lang.Number]) {{ {body} }}");
        assert!(compile_sources_with_api(&[&source], initial).is_err());
        let unit = compile_sources_with_api(&[&source], available).unwrap();
        assert_eq!(
            unit,
            parse_unit_ir_with_api(&unit.to_ir_text(), available).unwrap()
        );
        assert!(parse_unit_ir_with_api(&unit.to_ir_text(), initial).is_err());
    }
}

#[test]
fn tuple_self_appends_bound_flat_and_nested_signature_growth() {
    for append in ["x.addAll(x);", "x.add(x);"] {
        let source = format!(
            "function f() {{ var x=[1,\"seed\"]; {} return x.size(); }}",
            append.repeat(16)
        );
        let error = compile_sources(&[&source]).unwrap_err();
        assert!(
            error.to_string().contains("derived-signature budget"),
            "{error}"
        );
    }
}

#[test]
fn shape_loss_retains_native_element_unions_through_loops_and_saved_ir() {
    round_trip(
        r#"
        using Toybox.Lang;
        function unknown(x as [Lang.Number, Lang.String], tail as Lang.Array<Lang.Number>) as Lang.Array<Lang.Number or Lang.String> { x.addAll(tail); return x; }
        function forLoop(x as [Lang.Number, Lang.String], n as Lang.Number) as Lang.Array<Lang.Number or Lang.String> { for(var i=0;i<n;i++) { x.add("new"); } return x; }
        function whileLoop(x as [Lang.Number, Lang.String], n as Lang.Number) as Lang.Number { while(n>0) { x.add("new"); n--; } return x.size(); }
        function doLoop(x as [Lang.Number, Lang.String], n as Lang.Number) as Lang.Number { do { x.add(7); n--; } while(n>0); return x.size(); }
        function twoWrites(x as [Lang.Number, Lang.String], n as Lang.Number) as Lang.Array<Lang.Number or Lang.String or Lang.Boolean or Lang.Char> { for(var i=0;i<n;i++) { x.add('a'); x.add(true); } return x; }
        function allLoop(x as [Lang.Number, Lang.String], n as Lang.Number) as Lang.Number { for(var i=0;i<n;i++) { x.addAll([2,"tail"]); } return x.size(); }
        function indexed(x as [Lang.Number, Lang.String], i as Lang.Number) as Lang.Array<Lang.Number or Lang.String or Lang.Boolean> { x[i]=false; return x; }
        function indexedThenAppend(x as [Lang.Number, Lang.String], i as Lang.Number) as Lang.Number { x[i]=false; x.add(true); return x.size(); }
        function erasedThenWrite(x as [Lang.Number, Lang.String], tail as Lang.Array<Lang.Number>) as Lang.Number { x.addAll(tail); x[0]=7; return x.size(); }
        function branch(x as [Lang.Number, Lang.String], tail as Lang.Array<Lang.Number>, flag as Lang.Boolean) as Lang.Array<Lang.Number or Lang.String> { if(flag) { x.addAll(tail); } return x; }
        function nestedLoop(x as [Lang.Number, Lang.String], n as Lang.Number) as Lang.Number { for(var i=0;i<n;i++) { for(var j=0;j<n;j++) { x.add(7); } } return x.size(); }
        function nullableElements(x as [Lang.Number, Lang.String or Null], tail as Lang.Array<Lang.Boolean>) as Lang.Number { x.addAll(tail); return x.size(); }
    "#,
    );
}

#[test]
fn shape_loss_rejects_stale_slots_erasure_and_invariant_array_mutation() {
    for source in [
        r#"function f(x as [Toybox.Lang.Number,Toybox.Lang.String], i as Toybox.Lang.Number) as [Toybox.Lang.Number,Toybox.Lang.String] { x[i]=2; return x; }"#,
        r#"function f(x as [Toybox.Lang.Number,Toybox.Lang.String], i as Toybox.Lang.Number) as Toybox.Lang.Number { x[i]=2; return x[0]; }"#,
        r#"function f(x as Toybox.Lang.Array<Toybox.Lang.Number>) { x.add("wrong"); }"#,
        r#"function f(x as [Toybox.Lang.Number,Toybox.Lang.String], tail as Toybox.Lang.Array<Toybox.Lang.Number>) { x.addAll(tail); x.add(true); }"#,
        r#"function f(x as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Array<Toybox.Lang.Number or Toybox.Lang.String> { return x; }"#,
        r#"class A {} function f(x as [A,Toybox.Lang.String], tail as Toybox.Lang.Array<Toybox.Lang.Number>) { x.addAll(tail); }"#,
    ] {
        assert!(compile_sources(&[source]).is_err(), "accepted {source}");
    }
}

#[test]
fn nullable_tuple_mutation_uses_live_storage_guards() {
    round_trip(
        r#"
        using Toybox.Lang;
        typedef Pair as [Lang.Number, Lang.String];
        function append(x as Pair or Null) as Lang.Number { if(x != null) { x.add(true); return x.size(); } return 0; }
        function replace(x as Pair or Null) as Lang.Boolean { if(x != null) { x[0]=false; return x[0]; } return true; }
        function indexed(x as Pair or Null, i as Lang.Number) as Lang.Number { if(x != null) { x[i]=false; return x.size(); } return 0; }
        function local(flag as Lang.Boolean) as Lang.Number { var x=flag ? [1,"old"] : null; if(x != null) { x.add(true); return x.size(); } return 0; }
        function guardedLoop(x as Pair or Null, n as Lang.Number) as Lang.Number { if(x != null) { for(var i=0;i<n;i++) { x.add(true); } return x.size(); } return 0; }
        function pending(x as Pair or Null) as Lang.Number { var copy; copy=x; if(copy!=null) { copy.add(true); return copy.size(); } return 0; }
    "#,
    );
    for source in [
        r#"function f(x as [Toybox.Lang.Number] or Null) { x.add(true); }"#,
        r#"function f(x as [Toybox.Lang.Number] or Null) { x[0]=true; }"#,
    ] {
        assert!(compile_sources(&[source]).is_err(), "accepted {source}");
    }
}

#[test]
fn union_array_saved_reads_cannot_forge_invariant_storage_views() {
    let text = "ciq-ir\n.function f parameters=1 parameter-types=integer-array result=union(integer|string)-array\n.parameter %0 0\n.variable $0 %0\n.read-variable %1 $0 array-union(integer|string)\n.return %1\n.end\n";
    assert!(ciq_compiler::parse_function_ir(text).is_err());
    let source = r#"function f(x as [Toybox.Lang.Number,Toybox.Lang.String], n as Toybox.Lang.Number) as Toybox.Lang.Number { for(var i=0;i<n;i++) { x.add(7); } return x.size(); }"#;
    let unit = round_trip(source);
    let saved = unit.to_ir_text();
    assert!(saved.contains(".array-add "));
    let forged = saved.replace("array-union(integer|string)", "tuple(integer;string)");
    assert_ne!(saved, forged);
    assert!(parse_unit_ir(&forged).is_err());
}

#[test]
fn computed_and_nested_receivers_preserve_results_and_parent_alias_limits() {
    round_trip(
        r#"
        using Toybox.Lang;
        typedef Pair as [Lang.Number,Lang.String];
        class Token { function value() as Lang.Number { return 41; } }
        function pair() as Pair { return [1,"seed"]; }
        function add() as [Lang.Number,Lang.String,Lang.Boolean] { return pair().add(true); }
        function all() as [Lang.Number,Lang.String,Lang.Number,Lang.String] { return pair().addAll([2,"tail"]); }
        function source() as Lang.Number { return pair().add(new Token())[2].value(); }
        function empty() as Pair { return pair().addAll([]); }
        function chain() as [Lang.Number,Lang.String,Lang.Number,Lang.Boolean] { return pair().add(2).add(true); }
        function branch(flag as Lang.Boolean) as Lang.Number { return (flag ? pair() : [2,"other"]).add(true).size(); }
        function nestedResult(x as [Pair,Lang.Boolean]) as [Lang.Number,Lang.String,Lang.Boolean] { return x[0].add(true); }
        function nestedAlias(x as [Pair,Lang.Boolean]) as Pair { x[0].add(2); return x[0]; }
        function changed(x as [Pair,Lang.Boolean]) as Lang.Number { x[0][0]="changed"; return x[0][0]; }
        function compound(x as [Pair,Lang.Boolean]) as Lang.Number { x[0][0]+=2; x[0][0]++; return x[0][0]; }
        function freshWrite() as Void { pair()[0]=2; }
        function matrix() as [Pair,Pair] { return [[1,"left"],[2,"right"]]; }
        function index() as Lang.Number { return 0; }
        function value() as Lang.Number { return 7; }
        function captured() as Void { matrix()[index()][0]=value(); }
    "#,
    );
}
