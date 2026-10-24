using Toybox.Application;
using Toybox.System;
module NullableArrays {
    function booleans() as Toybox.Lang.Array<Toybox.Lang.Boolean or Null> {
        return [false, null, true];
    }
    function chars() as Toybox.Lang.Array<Toybox.Lang.Char or Null> {
        return ['A', null, 'C'];
    }
    function emptyBoolean() as Toybox.Lang.Array<Toybox.Lang.Boolean or Null> { return []; }
    function nullChar() as Toybox.Lang.Array<Toybox.Lang.Char or Null> { return [null]; }
    function booleanValue(value as Toybox.Lang.Boolean or Null) as Toybox.Lang.Boolean or Null {
        System.println("boolean"); return value;
    }
    function charValue(value as Toybox.Lang.Char or Null) as Toybox.Lang.Char or Null {
        System.println("char"); return value;
    }
    function computedBoolean(choose as Toybox.Lang.Boolean)
        as Toybox.Lang.Array<Toybox.Lang.Boolean or Null> {
        return [choose ? NullableArrays.booleanValue(false) : null,
                null, NullableArrays.booleanValue(true)];
    }
    function computedChar(choose as Toybox.Lang.Boolean)
        as Toybox.Lang.Array<Toybox.Lang.Char or Null> {
        return [choose ? NullableArrays.charValue('B') : null,
                null, NullableArrays.charValue('D')];
    }
    function showBoolean(value as Toybox.Lang.Boolean or Null) as Void {
        if (value != null) { System.println(value); } else { System.println("null"); }
    }
    function showChar(value as Toybox.Lang.Char or Null) as Void {
        if (value != null) { System.println(value.toNumber()); } else { System.println("null"); }
    }
    function scalarArrays() as Void {
        var b = NullableArrays.booleans();
        System.println(b.toString());
        NullableArrays.showBoolean(b[0]);
        NullableArrays.showBoolean(b[1]);
        var shared = b;
        shared[0] = null;
        b[1] = false;
        b.add(null).addAll([true, null]);
        System.println(b.toString());
        System.println(b.indexOf(null));
        System.println(b.remove(null));
        System.println(b.removeAll(null));
        System.println(b.reverse().toString());
        System.println(b.slice(0, 2).toString());
        var before = b[0];
        b[0] = null;
        NullableArrays.showBoolean(before);
        NullableArrays.showBoolean(b[0]);
        NullableArrays.showBoolean(NullableArrays.booleans()[0]);
        b = [];
        System.println(b.size());
        System.println(NullableArrays.emptyBoolean().size());
        System.println(NullableArrays.computedBoolean(true).toString());
        System.println(NullableArrays.computedBoolean(false).toString());
        var c = NullableArrays.chars();
        NullableArrays.showChar(c[0]);
        NullableArrays.showChar(c[1]);
        var saved = c[0];
        c[0] = null;
        c[1] = 'B';
        c.add(null).addAll(['D', null]);
        System.println(c.indexOf(null));
        System.println(c.remove(null));
        System.println(c.removeAll(null));
        NullableArrays.showChar(c.reverse()[0]);
        NullableArrays.showChar(c.slice(0, 2)[1]);
        NullableArrays.showChar(saved);
        NullableArrays.showChar(NullableArrays.chars()[0]);
        NullableArrays.showChar(NullableArrays.nullChar()[0]);
        var computed = NullableArrays.computedChar(true);
        NullableArrays.showChar(computed[0]);
        NullableArrays.showChar(computed[1]);
        NullableArrays.showChar(computed[2]);
        computed = NullableArrays.computedChar(false);
        NullableArrays.showChar(computed[0]);
        NullableArrays.showChar(computed[2]);
    }
    function numbers() as Toybox.Lang.Array<Toybox.Lang.Number or Null> {
        return [1, null, 3];
    }
    function strings() as Toybox.Lang.Array<Toybox.Lang.String or Null> {
        return ["one", null, "three"];
    }
    function empty() as Toybox.Lang.Array<Toybox.Lang.Number or Null> { return []; }
    function onlyNull() as Toybox.Lang.Array<Toybox.Lang.String or Null> { return [null]; }
    function computed(choose as Toybox.Lang.Boolean, n as Toybox.Lang.Number or Null)
        as Toybox.Lang.Array<Toybox.Lang.Number or Null> {
        return [choose ? n : null, 2, null];
    }
    function text(s as Toybox.Lang.String or Null) as Toybox.Lang.String or Null {
        System.println("text"); return s;
    }
    function computedText(choose as Toybox.Lang.Boolean)
        as Toybox.Lang.Array<Toybox.Lang.String or Null> {
        return [choose ? NullableArrays.text("yes") : null, null,
                NullableArrays.text("tail")];
    }
    function showNumber(n as Toybox.Lang.Number or Null) as Void {
        if (n != null) { System.println(n); } else { System.println("null"); }
    }
    function showString(s as Toybox.Lang.String or Null) as Void {
        if (s != null) { System.println(s); } else { System.println("null"); }
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        var a = NullableArrays.numbers();
        System.println(a.toString());
        NullableArrays.showNumber(a[0]);
        NullableArrays.showNumber(a[1]);
        a[0] = null;
        a[1] = 2;
        a.add(null).add(4).addAll([5, null]);
        System.println(a.toString());
        System.println(a.indexOf(null));
        System.println(a.remove(null));
        System.println(a.removeAll(null));
        System.println(a.toString());
        System.println(a.reverse().toString());
        System.println(a.slice(null, 2).toString());
        var before = a[0];
        a[0] = null;
        NullableArrays.showNumber(before);
        NullableArrays.showNumber(a[0]);
        a = [];
        System.println(a.size());
        System.println(NullableArrays.empty().size());
        System.println(NullableArrays.computed(true, null).toString());
        System.println(NullableArrays.computed(false, 7).toString());
        var s = NullableArrays.strings();
        var shared = s;
        shared[0] = null;
        s[1] = "two";
        s.add(null).addAll(["four", null]);
        System.println(s.toString());
        NullableArrays.showString(s[0]);
        NullableArrays.showString(s[1]);
        System.println(s.indexOf(null));
        System.println(s.remove(null));
        System.println(s.removeAll(null));
        System.println(s.reverse().toString());
        System.println(s.slice(0, 2).toString());
        System.println(NullableArrays.onlyNull().toString());
        System.println(NullableArrays.computedText(true).toString());
        System.println(NullableArrays.computedText(false).toString());
        var first = NullableArrays.numbers();
        first[0] = null;
        NullableArrays.showNumber(NullableArrays.numbers()[0]);
        NullableArrays.scalarArrays();
        NullArrays.run();
        System.exit();
    }
}

module NullArrays {
    function value() as Null { return null; }
    function literal() as Toybox.Lang.Array<Null> { return [null, null]; }
    function computed() as Toybox.Lang.Array<Null> { return [NullArrays.value()]; }
    function empty() as Toybox.Lang.Array<Null> { return []; }
    function allocated() as Toybox.Lang.Array<Null> { return new[2]; }
    function shape(x as [Null], i as Toybox.Lang.Number) as Toybox.Lang.Array<Null> {
        x[i] = null;
        return x;
    }
    function tail(x as [Null], y as Toybox.Lang.Array<Null>) as Toybox.Lang.Array<Null> {
        x.addAll(y);
        return x;
    }
    function run() as Void {
        System.println("null-elements");
        var x = NullArrays.literal();
        System.println(x[0] == null);
        x[0] = NullArrays.value();
        x.add(null).addAll(NullArrays.computed());
        System.println(x.size());
        System.println(x.indexOf(null));
        System.println(x.reverse()[0] == null);
        System.println(x.slice(0, 2).size());
        System.println(x.remove(null));
        System.println(x.size());
        System.println(x.removeAll(null));
        System.println(x.size());
        System.println(x.remove(null));
        System.println(NullArrays.empty().size());
        System.println(NullArrays.allocated()[1] == null);
        System.println(NullArrays.shape([null], 0).size());
        System.println(NullArrays.tail([null], NullArrays.computed()).size());
    }
}
