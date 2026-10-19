using Toybox.Lang;
using Toybox.System;
module Containers {
    function numbers() as Lang.Array<Lang.Numeric> { return [3, 1l, 2.0, 4.0d]; }
    function integers() as Lang.Array<Lang.Integer> { return [3, 1l, 2]; }
    function decimals() as Lang.Array<Lang.Decimal> { return [3.5, 1.5d, 2.5]; }
    function nullableNumbers() as Lang.Array<Lang.Numeric or Null> { return [1, null, 2l, 3.5, 4.5d]; }
    function nullableIntegers() as Lang.Array<Lang.Integer or Null> { return [1, null, 2l]; }
    function nullableDecimals() as Lang.Array<Lang.Decimal or Null> { return [1.5, null, 2.5d]; }
    function empty() as Lang.Array<Lang.Numeric> { return []; }
    function sized() as Lang.Array<Lang.Numeric or Null> { return new [2]; }
    function effect(n as Lang.Numeric) as Lang.Numeric { System.println("effect=" + n); return n; }
    function computed(flag as Lang.Boolean) as Lang.Array<Lang.Numeric> {
        return [Containers.effect(1), flag ? Containers.effect(2l) : Containers.effect(3.5d)];
    }
    function mapping() as Lang.Dictionary<Lang.Numeric, Lang.Numeric> { return {1 => 10, 2l => 20l, 3.0 => 30.5, 4.0d => 40.5d}; }
    function integerMap() as Lang.Dictionary<Lang.Integer, Lang.Integer> { return {1 => 10l, 2l => 20}; }
    function decimalMap() as Lang.Dictionary<Lang.Decimal, Lang.Decimal> { return {1.5 => 10.5d, 2.5d => 20.5}; }
    function nullableMap() as Lang.Dictionary<Lang.String, Lang.Numeric or Null> { return {"number" => 7, "long" => 8l, "float" => 9.5, "double" => 10.5d, "null" => null}; }
    function nullableIntegerMap() as Lang.Dictionary<Lang.Integer, Lang.Integer or Null> { return {1 => 10l, 2l => null}; }
    function nullableDecimalMap() as Lang.Dictionary<Lang.Decimal, Lang.Decimal or Null> { return {1.5 => 10.5d, 2.5d => null}; }
    function sameKeys() as Lang.Dictionary<Lang.Numeric, Lang.Integer> { return {1 => 1, 1l => 2, 1.0 => 3, 1.0d => 4}; }
    function emptyMap() as Lang.Dictionary<Lang.Numeric, Lang.Numeric> { return {}; }
    function computedMap(flag as Lang.Boolean) as Lang.Dictionary<Lang.Numeric, Lang.Numeric> {
        return {Containers.effect(5l) => flag ? Containers.effect(50.5) : Containers.effect(60.5d)};
    }
    function show(n as Lang.Numeric or Null) as Void {
        if (n == null) { System.println("null"); return; }
        if (n instanceof Lang.Number) { System.println("N:" + n); return; }
        if (n instanceof Lang.Long) { System.println("L:" + n); return; }
        if (n instanceof Lang.Float) { System.println("F:" + n); return; }
        System.println("D:" + n);
    }
    function arrays() as Void {
        var n = Containers.numbers();
        var i = Containers.integers();
        var d = Containers.decimals();
        n.sort(null); i.sort(null); d.sort(null);
        Containers.show(n[0]); Containers.show(n[1]); Containers.show(n[2]); Containers.show(n[3]);
        Containers.show(i[0]); Containers.show(d[0]);
        n[0] += 2.5d; ++i[0]; d[0] *= 2;
        Containers.show(n[0]); Containers.show(i[0]); Containers.show(d[0]);
        System.println(n.add(9l).indexOf(9l));
        System.println(i.addAll([7l, 8]).size());
        System.println(d.addAll([7.5d, 8.5]).size());
        System.println(n.remove(9l)); System.println(i.removeAll(7l));
        Containers.show(d.reverse().slice(0, 1)[0]);
        Containers.show(Containers.numbers()[0]);
        var maybe = Containers.nullableNumbers();
        var saved = maybe[2]; maybe[2] = null;
        if (saved != null) { Containers.show(saved + 1); }
        System.println(maybe.indexOf(null));
        System.println(maybe.remove(null));
        System.println(maybe.addAll([null, 5.5d]).size());
        var mi = Containers.nullableIntegers(); var md = Containers.nullableDecimals();
        var iv = mi[2]; var dv = md[2];
        if (iv != null) { Containers.show(iv & 3); }
        if (dv != null) { Containers.show(dv + 1); }
        mi[0] = null; md[0] = null;
        System.println(mi[0] == null); System.println(md[0] == null);
        var sized = Containers.sized();
        System.println(sized[0] == null); sized[0] = 7l;
        Containers.show(sized[0]);
        System.println(Containers.empty().add(1.5).size());
        var made = Containers.computed(true); Containers.show(made[1]);
        made = Containers.computed(false); Containers.show(made[1]);
    }
    function maps() as Void {
        var n = Containers.mapping(); var i = Containers.integerMap(); var d = Containers.decimalMap();
        Containers.show(n[1]); Containers.show(n[2l]); Containers.show(n[3.0]); Containers.show(n[4.0d]);
        Containers.show(n[99]); Containers.show(i[1]); Containers.show(d[1.5]);
        n[1] = 11.5d; i.put(1, 11); d.put(1.5, 11.5);
        Containers.show(n.get(1)); Containers.show(i.get(1)); Containers.show(d.get(1.5));
        System.println(n.keys().indexOf(2l) >= 0); System.println(n.values().indexOf(11.5d) >= 0);
        System.println(i.keys().size()); System.println(d.values().size());
        var maybe = Containers.nullableMap(); var saved = maybe["long"]; maybe["long"] = null;
        if (saved != null) { Containers.show(saved + 1); }
        Containers.show(maybe["long"]); Containers.show(maybe["null"]); Containers.show(maybe["missing"]);
        System.println(maybe.values().indexOf(null) >= 0);
        var mi = Containers.nullableIntegerMap(); var md = Containers.nullableDecimalMap();
        Containers.show(mi[1]); Containers.show(mi[2l]); Containers.show(md[1.5]); Containers.show(md[2.5d]);
        var same = Containers.sameKeys();
        System.println(same.size()); Containers.show(same[1]); Containers.show(same[1l]); Containers.show(same[1.0]); Containers.show(same[1.0d]);
        var empty = Containers.emptyMap(); empty.put(7.0d, 70l); Containers.show(empty[7.0d]);
        var made = Containers.computedMap(true); Containers.show(made[5l]);
        made = Containers.computedMap(false); Containers.show(made[5l]);
        n.remove(1); System.println(n.hasKey(1));
    }
    function run() as Void { Containers.arrays(); Containers.maps(); }
}
class MinimalConsoleApp extends Toybox.Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Containers.run(); System.exit(); }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
