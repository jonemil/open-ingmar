using Toybox.Lang;
using Toybox.System;
module UnionOps {
    function kind(n as Lang.Numeric) as Lang.String {
        if (n instanceof Lang.Number) { return "N"; }
        if (n instanceof Lang.Long) { return "L"; }
        if (n instanceof Lang.Float) { return "F"; }
        return "D";
    }
    function pair(a as Lang.Numeric, b as Lang.Numeric) as Void {
        var sum = a + b;
        System.println("sum=" + sum + ":" + UnionOps.kind(sum));
        System.println((a - b).toString());
        System.println((a * b).toString());
        System.println((a / b).toString());
        System.println(a < b);
    }
    function bits(a as Lang.Integer, b as Lang.Integer) as Lang.Integer {
        return ((a & b) | (a ^ b)) % 7;
    }
    function shift(a as Lang.Integer) as Lang.Integer { return (~a >> 1) << 1; }
    function decimal(a as Lang.Decimal, b as Lang.Integer) as Lang.Decimal {
        return a + b;
    }
    function negated(n as Lang.Numeric) as Lang.Numeric { return -n; }
    function optional(flag as Lang.Boolean) as Lang.Numeric or Null {
        return flag ? 8l : null;
    }
    function nullable(n as Lang.Numeric or Null) as Lang.Numeric or Null {
        if (n == null) { return null; }
        return n + 2;
    }
    function nullableInteger(n as Lang.Integer or Null) as Lang.Integer or Null {
        return n != null ? n + 1 : null;
    }
    function nullableDecimal(n as Lang.Decimal or Null) as Lang.Decimal or Null {
        return n != null ? n * 2 : null;
    }
    function effect(n as Lang.Numeric) as Lang.Numeric { System.println(n); return n; }
    function run() as Void {
        UnionOps.pair(3, 2);
        UnionOps.pair(3, 2l);
        UnionOps.pair(3, 2.0);
        UnionOps.pair(3, 2.0d);
        UnionOps.pair(3l, 2);
        UnionOps.pair(3l, 2l);
        UnionOps.pair(3l, 2.0);
        UnionOps.pair(3l, 2.0d);
        UnionOps.pair(3.0, 2);
        UnionOps.pair(3.0, 2l);
        UnionOps.pair(3.0, 2.0);
        UnionOps.pair(3.0, 2.0d);
        UnionOps.pair(3.0d, 2);
        UnionOps.pair(3.0d, 2l);
        UnionOps.pair(3.0d, 2.0);
        UnionOps.pair(3.0d, 2.0d);
        System.println(UnionOps.bits(13, 6l).toString());
        System.println(UnionOps.shift(13l).toString());
        System.println(UnionOps.decimal(1.5, 2l).toString());
        System.println(UnionOps.negated(4).toString());
        System.println(UnionOps.negated(4l).toString());
        System.println(UnionOps.negated(4.5).toString());
        System.println(UnionOps.negated(4.5d).toString());
        var n = UnionOps.optional(true);
        System.println(n != null && n > 0);
        var result = UnionOps.nullable(n);
        if (result != null) { System.println(result.toString()); }
        n = null;
        System.println(UnionOps.nullable(n) == null);
        var i = UnionOps.nullableInteger(3);
        if (i != null) { System.println(i.toString()); }
        var d = UnionOps.nullableDecimal(1.5d);
        if (d != null) { System.println(d.toString()); }
        var local = UnionOps.effect(2);
        local += UnionOps.effect(3l);
        local++;
        --local;
        System.println(local.toString());
        var bytes = [255, 255, 255, 255]b;
        var decoded = bytes.decodeNumber(Lang.NUMBER_FORMAT_UINT32, {});
        System.println((decoded + 1).toString());
    }
}
class MinimalConsoleApp extends Toybox.Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { UnionOps.run(); NativeUnions.run(); System.exit(); }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}

module NativeUnions {
    function take(x as Lang.Number or Lang.String) as Lang.Number {
        if (x instanceof Lang.Number) { return x + 1; }
        else { return x.length(); }
    }
    function common(x as Lang.Number or Lang.String) as Lang.String {
        return x.toString();
    }
    function nullable(x as Null or Lang.String or Lang.Number) as Lang.Number {
        if (x == null) { return 0; }
        if (x instanceof Lang.String) { return x.length(); }
        return x + 1;
    }
    function plus(x as Lang.Number or Lang.String) as Lang.Number or Lang.String {
        return x + 1;
    }
    function arrayOrText(x as Lang.Array<Lang.Number> or Lang.String) as Lang.Number {
        if (x instanceof Lang.Array) { return x[0]; }
        return x.length();
    }
    function optionalArray(x as Lang.Array<Lang.Number> or Null) as Lang.Number {
        if (x != null) { return x[0]; }
        return 0;
    }
    function loop(x as Lang.Number or Lang.String) as Lang.Number {
        var current = x;
        while (current instanceof Lang.Number) { current = "done"; }
        return current.length();
    }
    function zero(x as Null) as Null { return x; }
    function run() as Void {
        System.println("native-unions");
        System.println(NativeUnions.take(41));
        System.println(NativeUnions.take("four"));
        System.println(NativeUnions.common(9));
        System.println(NativeUnions.common("ok"));
        System.println(NativeUnions.nullable(null));
        System.println(NativeUnions.nullable(4));
        System.println(NativeUnions.nullable("abc"));
        System.println(NativeUnions.plus(41));
        System.println(NativeUnions.plus("x"));
        System.println(NativeUnions.arrayOrText([42]));
        System.println(NativeUnions.arrayOrText("abc"));
        System.println(NativeUnions.optionalArray([7]));
        System.println(NativeUnions.optionalArray(null));
        System.println(NativeUnions.loop(1));
        System.println(NativeUnions.loop("hi"));
        System.println(NativeUnions.zero(null) == null);
        System.println('A' == null);
    }
}
