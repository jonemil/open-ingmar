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
    function onStart(state) { UnionOps.run(); NativeUnions.run(); SourceUnions.run(); MixedUnions.run(); System.exit(); }
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

class UnionToken extends Lang.Object {
    function initialize() {}
    function count() as Lang.Number { return 7; }
    function compare(a as Lang.Object, b as Lang.Object) as Lang.Number {
        return (a as Lang.Number) - (b as Lang.Number);
    }
}
class UnionMarker extends Lang.Object {
    function initialize() {}
    function size() as Lang.Number { return 9; }
    function compare(a as Lang.Object, b as Lang.Object) as Lang.Number {
        return (b as Lang.Number) - (a as Lang.Number);
    }
}
module SourceUnions {
    function pick(flag as Lang.Boolean) as UnionToken or UnionMarker {
        return flag ? new UnionToken() : new UnionMarker();
    }
    function echo(value as UnionMarker or UnionToken) as UnionToken or UnionMarker {
        return value;
    }
    function measure(value as UnionToken or UnionMarker or Null) as Lang.Number {
        if (value == null) { return 0; }
        if (value instanceof UnionToken) { return value.count(); }
        return value.size();
    }
    function changing(flag as Lang.Boolean) as Lang.Number {
        var value = new UnionToken();
        if (flag) { value = new UnionMarker(); }
        if (!(value instanceof UnionMarker)) { return value.count(); }
        return value.size();
    }
    function loop() as Lang.Number {
        var value = SourceUnions.pick(true);
        while (value instanceof UnionToken) { value = new UnionMarker(); }
        return value.size();
    }
    function sorted(flag as Lang.Boolean) as Lang.Number {
        var values = [3, 1, 2];
        values.sort(SourceUnions.pick(flag));
        return values[0];
    }
    function run() as Void {
        System.println("source-unions");
        System.println(SourceUnions.measure(SourceUnions.echo(SourceUnions.pick(true))));
        System.println(SourceUnions.measure(SourceUnions.pick(false)));
        System.println(SourceUnions.measure(null));
        System.println(SourceUnions.changing(true));
        System.println(SourceUnions.changing(false));
        System.println(SourceUnions.loop());
        System.println(SourceUnions.sorted(true));
        System.println(SourceUnions.sorted(false));
    }
}

module MixedUnions {
    function pick(flag as Lang.Boolean) as UnionToken or Lang.String {
        return flag ? new UnionToken() : "four";
    }
    function echo(value as UnionToken or Lang.String) as Lang.String or UnionToken {
        return value;
    }
    function measure(value as UnionToken or Lang.String or Null) as Lang.Number {
        if (value == null) { return 0; }
        if (value instanceof UnionToken) { return value.count(); }
        return value.length();
    }
    function numeric(value as UnionToken or Lang.Number) as Lang.Number {
        if (value instanceof Lang.Number) { return value + 1; }
        return value.count();
    }
    function indexed(value as UnionToken or Lang.Array<Lang.Number>) as Lang.Number {
        if (value instanceof UnionToken) { return value.count(); }
        return value[0];
    }
    function loop() as Lang.Number {
        var value = MixedUnions.pick(true);
        while (value instanceof UnionToken) { value = "done"; }
        return value.length();
    }
    function recover(value as UnionToken or Lang.Object) as Lang.Number {
        if (value instanceof UnionMarker) { return value.size(); }
        if (value instanceof UnionToken) { return value.count(); }
        return 0;
    }
    function sorted(value as UnionMarker or Lang.Comparator) as Lang.Number {
        if (value instanceof UnionMarker) { return value.size(); }
        var values = [3, 1, 2];
        values.sort(value);
        return values[0];
    }
    function run() as Void {
        System.println("mixed-unions");
        System.println(MixedUnions.measure(MixedUnions.echo(MixedUnions.pick(true))));
        System.println(MixedUnions.measure(MixedUnions.pick(false)));
        System.println(MixedUnions.measure(null));
        System.println(MixedUnions.numeric(41));
        System.println(MixedUnions.numeric(new UnionToken()));
        System.println(MixedUnions.indexed([42]));
        System.println(MixedUnions.indexed(new UnionToken()));
        System.println(MixedUnions.loop());
        System.println(MixedUnions.recover(new UnionMarker()));
        System.println(MixedUnions.sorted(new UnionToken()));
        System.println(MixedUnions.sorted(new UnionMarker()));
    }
}
