using Toybox.Lang;
using Toybox.System;
module Comparisons {
    function sign(n as Lang.Number) as Lang.Number {
        if (n < 0) { return -1; }
        if (n > 0) { return 1; }
        return 0;
    }
    function number(other as Lang.Object) as Lang.Number { return 2.compareTo(other); }
    function longValue(other as Lang.Object) as Lang.Number { return 2l.compareTo(other); }
    function floatValue(other as Lang.Object) as Lang.Number { return 2.0.compareTo(other); }
    function doubleValue(other as Lang.Object) as Lang.Number { return 2.0d.compareTo(other); }
    function booleanValue(other as Lang.Object) as Lang.Number { return true.compareTo(other); }
    function charValue(other as Lang.Object) as Lang.Number { return 'B'.compareTo(other); }
    function stringValue(other as Lang.Object) as Lang.Number { return "b".compareTo(other); }
    function numeric(a as Lang.Numeric, b as Lang.Numeric) as Lang.Number { return a.compareTo(b); }
    function optional(other as Lang.Object or Null) as Lang.Number {
        if (other == null) { return 0; }
        return Comparisons.number(other);
    }
    function row(other as Lang.Object) as Void {
        System.println("N=" + Comparisons.sign(Comparisons.number(other))
            + ",L=" + Comparisons.sign(Comparisons.longValue(other))
            + ",F=" + Comparisons.sign(Comparisons.floatValue(other))
            + ",D=" + Comparisons.sign(Comparisons.doubleValue(other))
            + ",B=" + Comparisons.sign(Comparisons.booleanValue(other))
            + ",C=" + Comparisons.sign(Comparisons.charValue(other)));
    }
    function values() as Lang.Array<Lang.Numeric> { return [3, 3l, 3.0, 3.0d]; }
    function equals(a as Lang.Object, b as Lang.Object or Null) as Lang.Boolean { return a.equals(b); }
    function run() as Void {
        Comparisons.row(1);
        Comparisons.row(2l);
        Comparisons.row(3.0);
        Comparisons.row(4.0d);
        Comparisons.row(true);
        Comparisons.row('A');
        System.println(Comparisons.sign(Comparisons.stringValue("a")));
        System.println(Comparisons.sign(Comparisons.stringValue("b")));
        System.println(Comparisons.sign(Comparisons.stringValue("c")));
        System.println(Comparisons.optional(null));
        System.println(Comparisons.optional(1.5d));
        var values = Comparisons.values();
        var i = 0;
        while (i < values.size()) {
            var j = 0;
            while (j < values.size()) {
                System.println(Comparisons.numeric(values[i], values[j]));
                j++;
            }
            i++;
        }
        var object = new Target();
        var view = new Toybox.WatchUi.View();
        System.println(2.equals(object));
        System.println(2.equals(view));
        System.println("text".equals(view));
        System.println([1, 2].equals(view));
        System.println({1 => 2}.equals(view));
        System.println([1, 2]b.equals(view));
        System.println(Comparisons.equals(object, object));
        System.println(Comparisons.equals(object, new Target()));
        System.println(Comparisons.equals(object, view));
        System.println(Comparisons.equals(object, null));
        System.println(Comparisons.equals(view, view));
        System.println(Comparisons.equals(2, 2));
        System.println(Comparisons.equals(2, 2l));
        System.println(Comparisons.equals(2, 2.0));
        System.println(Comparisons.equals(2, 2.0d));
        System.println(Comparisons.equals("text", "text"));
        System.println(Comparisons.equals("text", 2));
    }
}
class Target {
    function initialize() {}
    function value() { return 7; }
}
class MinimalConsoleApp extends Toybox.Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Comparisons.run(); System.exit(); }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
