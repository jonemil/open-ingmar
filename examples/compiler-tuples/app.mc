using Toybox.Lang;
using Toybox.System;

typedef Pair as [Lang.Number, Lang.String];
typedef Factory as Method() as Pair;

class Token {
    function number() as Lang.Number { return 41; }
}

class Holder {
    var pair as [Token, Lang.String];
    var optional as Pair or Null;
    function initialize() { self.pair = [new Token(), "field"]; }
    function number() as Lang.Number { return self.pair[0].number(); }
    function text() as Lang.String { return self.pair[1]; }
    function set() as Void { self.optional = [7, "optional"]; }
    function optionalNumber() as Lang.Number {
        var pair = self.optional;
        if (pair != null) { return pair[0]; }
        return 0;
    }
}

class Worker {
    function pair() as Pair { return [42, "callback"]; }
}

module Tuples {
    function pair(n as Lang.Number) as Pair { return [n, "tuple"]; }
    function first(x as Pair) as Lang.Number { return x[0]; }
    function selected(x as Pair, i as Lang.Number) as Lang.Number or Lang.String {
        return x[i];
    }
    function nested() as [Lang.Number, [Lang.String]] { return [1, ["nested"]]; }
    function source() as [Token, Lang.String] { return [new Token(), "source"]; }
    function relay(x as [Token, Lang.String]) as [Token, Lang.String] { return x; }
    function invoke(factory as Factory) as Pair { return factory.invoke(); }
    function maybe(flag as Lang.Boolean) as Pair or Null {
        if (flag) { return [8, "nullable"]; }
        return null;
    }
    function condition(flag as Lang.Boolean) as Pair {
        return flag ? [9, "yes"] : [10, "no"];
    }
    function array(x as [Lang.Number]) as Lang.Array<Lang.Number> { return x; }
    function one() as [Lang.Number] { return [11]; }
    function number() as Lang.Number { System.println("first"); return 12; }
    function text() as Lang.String { System.println("second"); return "ordered"; }
    function ordered() as Pair { return [number(), text()]; }
}

module Inferred {
    function pair() as Lang.String { var value = [42, "inferred"]; return value[1]; }
    function widths() as Lang.Long { var value = [1, 2l, 3.0f, 4.0d]; return value[1]; }
    function charCode() as Lang.Number { var value = [true, 'a', "text"]; return value[1].toNumber(); }
    function nullable() as Lang.Boolean { var value = [null, "value"]; return value[0] == null; }
    function allNull() as Lang.Number { var value = [null]; return value.size(); }
    function nested() as Lang.String { var value = [[1, 2], [3, "nested-inferred"]]; return value[1][1]; }
    function source() as Lang.Number { var value = [new Token(), ["source-inferred"]]; return value[0].number(); }
    function ordered() as Lang.String {
        var value = [Tuples.number(), [Tuples.text()]];
        System.println(value[0]);
        return value[1][0];
    }
}

module Mutation {
    function replace(x as Pair) as [Lang.String, Lang.String] {
        x[0] = "replaced";
        return x;
    }
    function source() as Lang.Number {
        var x = [1, "old"];
        x[0] = new Token();
        return x[0].number();
    }
    function nested() as Lang.String {
        var x = [1, "old"];
        x[0] = [2, "nested-replacement"];
        return x[0][1];
    }
    function branch(flag as Lang.Boolean) as Lang.Number or Lang.String {
        var x = [1, "old"];
        if (flag) { x[0] = "branch-replacement"; }
        return x[0];
    }
    function compound(x as Pair) as Lang.Number {
        x[0] += 2;
        x[0]++;
        return x[0];
    }
    function conditional(x as Pair, flag as Lang.Boolean) as Lang.Number {
        x[0] = flag ? 42 : 7;
        return x[0];
    }
}

module Append {
    function value(x as [Lang.Number]) as [Lang.Number, Lang.String] {
        return x.add("appended");
    }
    function all(x as [Lang.Number], tail as [Token, Lang.String]) as Lang.Number {
        x.addAll(tail);
        return x[1].number();
    }
    function empty(x as Pair) as Lang.Number {
        x.addAll([]);
        return x.size();
    }
    function aliasResult() as Lang.Boolean {
        var x = [1, "old"];
        var y = x.add(true);
        return y[2];
    }
    function literal(x as [Lang.Number]) as [Lang.Number, Lang.Number, Lang.Number] {
        x.addAll([2, 3]);
        return x;
    }
    function branch(x as [Lang.Number], flag as Lang.Boolean) as Lang.Number {
        if (flag) { x.add("tail"); }
        return x.size();
    }
    function source(x as [Lang.Number]) as Lang.Number {
        x.add(new Token());
        return x[1].number();
    }
    function effects(x as [Lang.Number]) as Lang.String {
        x.add(Tuples.text());
        return x[1];
    }
    function nested(x as [Lang.Number]) as Lang.Number {
        x.add(x.add("tail").size());
        return x.size();
    }
    function mixed(x as [Lang.Number]) as Lang.Boolean {
        x.addAll(["tail", true]);
        return x[2];
    }
    function container(x as [Lang.Number]) as Lang.Number {
        x.add([]);
        return x[1].size();
    }
}

module Widen {
    function tail() as Lang.Array<Lang.Number> { return [7, 8]; }
    function all(x as Pair) as Lang.Array<Lang.Number or Lang.String> {
        x.addAll(tail());
        return x;
    }
    function forLoop(x as Pair, n as Lang.Number) as Lang.Array<Lang.Number or Lang.String> {
        for (var i = 0; i < n; i++) { x.add("new"); }
        return x;
    }
    function whileLoop(x as Pair, n as Lang.Number) as Lang.Number {
        while (n > 0) { x.add("new"); n--; }
        return x.size();
    }
    function doLoop(x as Pair, n as Lang.Number) as Lang.Number {
        do { x.add(7); n--; } while (n > 0);
        return x.size();
    }
    function many(x as Pair, n as Lang.Number) as Lang.Number {
        for (var i = 0; i < n; i++) { x.add('a'); x.add(true); }
        return x.size();
    }
    function indexed(x as Pair, i as Lang.Number) as Lang.Array<Lang.Number or Lang.String or Lang.Boolean> {
        x[i] = false;
        return x;
    }
    function afterIndexed(x as Pair, i as Lang.Number) as Lang.Number {
        x[i] = false;
        x.add(true);
        return x.size();
    }
    function guarded(x as Pair or Null, n as Lang.Number) as Lang.Number {
        if (x != null) {
            for (var i = 0; i < n; i++) { x.add(true); }
            return x.size();
        }
        return 0;
    }
    function local(flag as Lang.Boolean) as Lang.Number {
        var x = flag ? [1, "old"] : null;
        if (x != null) { x.add(true); return x.size(); }
        return 0;
    }
    function allLoop(x as Pair, n as Lang.Number) as Lang.Number {
        for (var i = 0; i < n; i++) { x.addAll([2, "tail"]); }
        return x.size();
    }
    function guardedReplace(x as Pair or Null) as Lang.Boolean {
        if (x != null) { x[0] = false; return x[0]; }
        return true;
    }
}

module Receivers {
    function pair() as Pair {
        System.println("computed-receiver");
        return [1, "seed"];
    }
    function flag() as Lang.Boolean {
        System.println("computed-argument");
        return false;
    }
    function value() as [Lang.Number, Lang.String, Lang.Boolean] {
        return pair().add(flag());
    }
    function all() as [Lang.Number, Lang.String, Lang.Number, Lang.String] {
        return pair().addAll([2, "tail"]);
    }
    function nested(x as [Pair, Lang.Boolean]) as [Lang.Number, Lang.String, Lang.Number] {
        return x[0].add(2);
    }
    function write(x as [Pair, Lang.Boolean]) as Void { x[0][0] = "nested-write"; }
    function compound(x as [Pair, Lang.Boolean]) as Lang.Number {
        x[0][0] += 2;
        x[0][0]++;
        return x[0][0];
    }
    function matrix() as [Pair, Pair] {
        System.println("matrix");
        return [[1, "left"], [2, "right"]];
    }
    function index() as Lang.Number { System.println("index"); return 0; }
    function number() as Lang.Number { System.println("value"); return 7; }
    function orderedWrite() as Void { matrix()[index()][0] = number(); }
    function chain() as [Lang.Number, Lang.String, Lang.Number, Lang.Boolean] {
        return pair().add(2).add(true);
    }
}

class TupleFieldBox {
    var pair as Pair;
    var optional as Pair or Null;
    function initialize() { self.pair = [1, "seed"]; }
    function write() as Lang.String { self.pair[0] = "field-write"; return self.pair[0]; }
    function append() as [Lang.Number, Lang.String, Lang.Boolean] {
        self.pair.add(true);
        return self.pair;
    }
    function branch(flag as Lang.Boolean) as Lang.Number or Lang.Boolean {
        if (flag) { self.pair[0] = true; }
        return self.pair[0];
    }
    function reset() as Void { self.pair = [7, "reset"]; }
    function afterReset() as Lang.Number {
        self.pair[0] = "changed";
        self.reset();
        return self.pair[0];
    }
    function afterAssign() as Lang.Number {
        self.pair[0] = "changed";
        self.pair = [7, "assigned"];
        return self.pair[0];
    }
    function repeated(n as Lang.Number) as Lang.Array<Lang.Number or Lang.String or Lang.Boolean> {
        for (var i = 0; i < n; i++) { self.pair.add(true); }
        return self.pair;
    }
    function setOptional() as Void { self.optional = [7, "optional"]; }
    function guarded(n as Lang.Number) as Lang.Number {
        if (self.optional != null) {
            for (var i = 0; i < n; i++) { self.optional.add(true); }
            return self.optional.size();
        }
        return 0;
    }
    function indexed(i as Lang.Number) as Lang.Array<Lang.Number or Lang.String or Lang.Boolean> {
        self.pair[i] = false;
        return self.pair;
    }
    function compound() as Lang.Number {
        self.pair[0] += 2;
        self.pair[0]++;
        return self.pair[0];
    }
    function resetAndValue() as Lang.Boolean { self.reset(); return true; }
    function captured() as Lang.Number {
        self.pair.add(self.resetAndValue());
        return self.pair.size();
    }
    function capturedResult() as Lang.Number {
        var old = self.pair.add(self.resetAndValue());
        return old.size();
    }
    function source() as Lang.Number {
        self.pair[0] = new Token();
        return self.pair[0].number();
    }
}

module FieldCases {
    function run() as Void {
        System.println(new TupleFieldBox().write());
        System.println(new TupleFieldBox().append()[2]);
        System.println(new TupleFieldBox().branch(true));
        System.println(new TupleFieldBox().branch(false));
        System.println(new TupleFieldBox().afterReset());
        System.println(new TupleFieldBox().afterAssign());
        System.println(new TupleFieldBox().repeated(0).size());
        System.println(new TupleFieldBox().repeated(2).size());
        var optional = new TupleFieldBox();
        System.println(optional.guarded(2));
        optional.setOptional();
        System.println(optional.guarded(2));
        System.println(new TupleFieldBox().indexed(0)[0]);
        System.println(new TupleFieldBox().compound());
        System.println(new TupleFieldBox().captured());
        System.println(new TupleFieldBox().capturedResult());
        System.println(new TupleFieldBox().source());
    }
}

class ExternalOwner {
    var count as Lang.Number;
    var pair as Pair;
    var token as Token or Null;
    function initialize() { self.count = 1; self.pair = [1, "external"]; }
}

module ExternalFields {
    function owner(x as ExternalOwner) as ExternalOwner {
        System.println("external-owner");
        return x;
    }
    function rhs() as Lang.Number { System.println("external-rhs"); return 2; }
    function run() as Void {
        var x = new ExternalOwner();
        owner(x).count += rhs();
        x.count++;
        x.count--;
        System.println(x.count);
        x.count = true ? 7 : 8;
        System.println(x.count);
        var y = x;
        x = new ExternalOwner();
        y.count = 9;
        System.println(x.count);
        System.println(y.count);
        x.token = new Token();
        var token = x.token;
        if (token != null) { System.println(token.number()); }
        x.token = null;
        System.println(x.token == null);
        x.pair = [2, "assigned"];
        x.pair[0] += 3;
        x.pair[0]++;
        System.println(x.pair[0]);
        System.println(x.pair.add(true).addAll([3, "tail"]).size());
        System.println(x.pair.size());
        var captured = new TupleFieldBox();
        System.println(captured.pair.add(captured.resetAndValue()).size());
        System.println(captured.pair.size());
        var source = new Holder();
        System.println(source.pair[0].number());
        for (var i = 0; i < 2; i++) { source.optional = [i, "loop"]; }
        var optional = source.optional;
        if (optional != null) { System.println(optional[0]); }
    }
}

typedef FieldRecord as interface { var pair as Pair; };
typedef OtherRecord as interface { var pair as Pair; };
typedef OptionalRecord as interface { var pair as Pair or Null; };
class OptionalFieldOwner { var pair as Pair or Null; }

module StructuralFields {
    function write(x as FieldRecord) as Lang.String {
        x.pair[0] = "structural";
        return x.pair[0];
    }
    function shared(x as FieldRecord, y as FieldRecord) as Lang.String {
        x.pair[0] = "shared";
        return y.pair[0];
    }
    function distinct(x as FieldRecord, y as OtherRecord) as Lang.Number {
        x.pair[0] = "distinct";
        return y.pair[0];
    }
    function aliased(x as FieldRecord) as Lang.String {
        var y = x;
        y.pair[0] = "alias";
        return x.pair[0];
    }
    function append(x as FieldRecord) as Lang.Number {
        x.pair.add(true);
        x.pair.addAll([2, "tail"]);
        return x.pair.size();
    }
    function branch(x as FieldRecord, flag as Lang.Boolean) as Lang.Number or Lang.Boolean {
        if (flag) { x.pair[0] = true; }
        return x.pair[0];
    }
    function replace(x as FieldRecord) as Lang.Number {
        x.pair[0] = "old";
        x.pair = [7, "replacement"];
        return x.pair[0];
    }
    function reset(x as FieldRecord) as Void { x.pair = [8, "reset"]; }
    function afterCall(x as FieldRecord) as Lang.Number {
        x.pair[0] = "old";
        reset(x);
        return x.pair[0];
    }
    function repeated(x as FieldRecord, n as Lang.Number)
        as Lang.Array<Lang.Number or Lang.String or Lang.Boolean> {
        for (var i = 0; i < n; i++) { x.pair.add(true); }
        return x.pair;
    }
    function optional(x as OptionalRecord) as Lang.Number {
        if (x.pair != null) { x.pair.add(true); return x.pair.size(); }
        return 0;
    }
    function run() as Void {
        System.println(write(new ExternalOwner()));
        var owner = new ExternalOwner();
        System.println(shared(owner, owner));
        System.println(shared(new ExternalOwner(), new ExternalOwner()));
        System.println(distinct(new ExternalOwner(), new ExternalOwner()));
        System.println(aliased(new ExternalOwner()));
        System.println(append(new ExternalOwner()));
        System.println(branch(new ExternalOwner(), true));
        System.println(branch(new ExternalOwner(), false));
        System.println(replace(new ExternalOwner()));
        System.println(afterCall(new ExternalOwner()));
        System.println(repeated(new ExternalOwner(), 0).size());
        System.println(repeated(new ExternalOwner(), 2).size());
        var selected = new OptionalFieldOwner();
        System.println(optional(selected));
        selected.pair = [1, "optional"];
        System.println(optional(selected));
    }
}

class MinimalConsoleApp extends Toybox.Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println("tuples");
        var pair = Tuples.pair(42);
        System.println(pair[0]);
        System.println(pair[1]);
        System.println(pair.size());
        System.println(Tuples.first([7, "argument"]));
        System.println(Tuples.selected(pair, 1));
        System.println(Tuples.nested()[1][0]);
        System.println(Tuples.relay(Tuples.source())[0].number());
        var holder = new Holder();
        System.println(holder.number());
        System.println(holder.text());
        System.println(holder.optionalNumber());
        holder.set();
        System.println(holder.optionalNumber());
        var worker = new Worker();
        System.println(Tuples.invoke(worker.method(:pair))[1]);
        var maybe = Tuples.maybe(true);
        if (maybe != null) { System.println(maybe[0]); }
        System.println(Tuples.condition(false)[1]);
        System.println(Tuples.array(Tuples.one())[0]);
        var ordered = Tuples.ordered();
        System.println(ordered[0]);
        System.println(ordered[1]);
        System.println(Inferred.pair());
        System.println(Inferred.widths());
        System.println(Inferred.charCode());
        System.println(Inferred.nullable());
        System.println(Inferred.allNull());
        System.println(Inferred.nested());
        System.println(Inferred.source());
        System.println(Inferred.ordered());
        System.println(Mutation.replace([1, "old"])[0]);
        System.println(Mutation.source());
        System.println(Mutation.nested());
        System.println(Mutation.branch(true));
        System.println(Mutation.branch(false));
        System.println(Mutation.compound([39, "compound"]));
        System.println(Mutation.conditional([0, "conditional"], true));
        System.println(Append.value([1])[1]);
        System.println(Append.all([1], [new Token(), "tail"]));
        System.println(Append.empty([1, "old"]));
        System.println(Append.aliasResult());
        System.println(Append.literal([1])[2]);
        System.println(Append.branch([1], true));
        System.println(Append.branch([1], false));
        System.println(Append.source([1]));
        System.println(Append.effects([1]));
        System.println(Append.nested([1]));
        System.println(Append.mixed([1]));
        System.println(Append.container([1]));
        var widened = Widen.all([1, "seed"]);
        System.println(widened.size());
        System.println(widened[1]);
        System.println(Widen.forLoop([1, "seed"], 3).size());
        System.println(Widen.forLoop([1, "seed"], 0).size());
        System.println(Widen.whileLoop([1, "seed"], 2));
        System.println(Widen.doLoop([1, "seed"], 0));
        System.println(Widen.many([1, "seed"], 2));
        System.println(Widen.indexed([1, "seed"], 0)[0]);
        System.println(Widen.afterIndexed([1, "seed"], 0));
        System.println(Widen.guarded([1, "seed"], 2));
        System.println(Widen.guarded(null, 2));
        System.println(Widen.local(true));
        System.println(Widen.local(false));
        System.println(Widen.allLoop([1, "seed"], 2));
        System.println(Widen.guardedReplace([1, "seed"]));
        System.println(Widen.guardedReplace(null));
        System.println(Receivers.value()[2]);
        System.println(Receivers.all()[3]);
        var nested = [[1, "seed"], true];
        System.println(Receivers.nested(nested)[2]);
        System.println(nested[0].size());
        var changed = [[1, "seed"], true];
        Receivers.write(changed);
        System.println(changed[0][0]);
        System.println(Receivers.compound([[1, "seed"], true]));
        Receivers.orderedWrite();
        var matrix = Receivers.matrix();
        matrix[Receivers.index()][0] = Receivers.number();
        System.println(matrix[0][0]);
        System.println(Receivers.chain()[3]);
        FieldCases.run();
        ExternalFields.run();
        StructuralFields.run();
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
