using Toybox.Application;
using Toybox.System;
module NullableArrays {
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
        System.exit();
    }
}
