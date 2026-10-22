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
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
