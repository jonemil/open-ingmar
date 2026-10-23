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
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
