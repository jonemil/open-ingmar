using Toybox.Application;
using Toybox.System;
using Toybox.Lang as L;
module Tests {
    function show(o as Toybox.Lang.Object) as Void {
        System.println(o instanceof L.Number);
        System.println(o instanceof L.String);
        System.println(o has :toString);
        System.println(o has :missing);
    }
    function absent(o as Toybox.Lang.Object or Null) as Void {
        System.println(o instanceof L.Number);
        if (o != null) { System.println(o has :toString); }
        else { System.println(false); }
    }
    function member(o as Toybox.Lang.Object, s as Toybox.Lang.Symbol) as Toybox.Lang.Boolean {
        return o has s;
    }
    function receiver() as Toybox.Lang.Object {
        System.println("receiver"); return "x";
    }
    function selector() as Toybox.Lang.Symbol {
        System.println("selector"); return :length;
    }
    function run() as Void {
        ImportedTypes.run();
        SourceTests.run();
        Lexical.run();
        Refine.run();
        Tests.show(3); Tests.show("x"); Tests.absent(null); Tests.absent("x");
        System.println(Tests.member("x", :length));
        System.println(Tests.receiver() has Tests.selector());
        System.println(3l instanceof L.Long);
        System.println(1.0 instanceof L.Float);
        System.println(1.0d instanceof L.Double);
        System.println(true instanceof L.Boolean);
        System.println('a' instanceof L.Char);
        var a = [1];
        System.println(a instanceof L.Array);
        System.println(a.weak() instanceof L.WeakReference);
        var d = {1 => 2};
        System.println(d instanceof L.Dictionary);
        System.println(:length instanceof L.Symbol);
        System.println("x".method(:toString) instanceof L.Method);
        System.println("x" instanceof L.Object);
        System.println(null instanceof L.Object);
        System.println(!(3 instanceof L.String));
        System.println(3 instanceof L.Number && "x" has :length);
        System.println(3 instanceof L.String || "x" has :length);
        var p = new Probe();
        System.println(p instanceof L.Object);
        System.println(p has :field);
        System.println(p has :value);
        System.println(p has :missing);
    }
}
class Probe {
    var field = 1;
    function initialize() {}
    function value() { return self.field; }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Tests.run(); System.exit(); }
}

module Refine {
    function number(o as Toybox.Lang.Object or Null) as Toybox.Lang.Number {
        if (o instanceof L.Number) { return o + 1; } else { return -1; }
    }
    function length(o as Toybox.Lang.Object) as Toybox.Lang.Number {
        if (o instanceof L.String) { return o.length(); } else { return 0; }
    }
    function positive(o as Toybox.Lang.Object) as Toybox.Lang.Boolean {
        return o instanceof L.Number && o > 0;
    }
    function inverted(o as Toybox.Lang.Object) as Toybox.Lang.Number {
        if (!(o instanceof L.Number)) { return 0; } else { return o * 2; }
    }
    function loop(o as Toybox.Lang.Object) as Toybox.Lang.Number {
        var value = o;
        var total = 0;
        while (value instanceof L.Number) { total = total + value; value = "done"; }
        return total;
    }
    function conditional(o as Toybox.Lang.Object) as Toybox.Lang.Number {
        return o instanceof L.String ? o.length() : 0;
    }
    function run() as Void {
        System.println(Refine.number(7));
        System.println(Refine.number(null));
        System.println(Refine.length("hello"));
        System.println(Refine.length(9));
        System.println(Refine.positive(3));
        System.println(Refine.positive("x"));
        System.println(Refine.inverted(4));
        System.println(Refine.loop(6));
        System.println(Refine.conditional("abc"));
        var values = [3, 1, 2];
        values.sort(new NumberOrder());
        System.println(values.toString());
    }
}
class NumberOrder {
    function initialize() {}
    function compare(a as Toybox.Lang.Object, b as Toybox.Lang.Object) as Toybox.Lang.Number {
        if (a instanceof L.Number && b instanceof L.Number) { return a - b; }
        else { return a.toString().compareTo(b.toString()); }
    }
}

module Lexical {
    function number(o as Toybox.Lang.Object) as Toybox.Lang.Number {
        return o as L.Number + 1;
    }
    function nullable(o as Toybox.Lang.Object or Null) as Toybox.Lang.String or Null {
        return o as L.String or Null;
    }
    function effect() as Toybox.Lang.Object { System.println("cast-effect"); return "abc"; }
    function run() as Void {
        System.println(Lexical.number(3));
        System.println(1 + 3 as L.Object as L.Number);
        System.println((3 as L.String) instanceof L.Number);
        System.println((null as L.String) instanceof L.String);
        System.println((3 as L.Double) instanceof L.Number);
        System.println(Lexical.nullable("x") as L.String);
        System.println(Lexical.nullable(null) == null);
        var erased = "hello" as L.Object;
        System.println((erased as L.String).length());
        System.println((true ? 1 : 2) as L.Number);
        System.println((Lexical.effect() as L.String).length());
        var array = [1];
        var weak = array.weak() as L.Object;
        System.println((weak as L.WeakReference).stillAlive());
        var dictionary = {1 => 2};
        System.println((dictionary as L.Object as L.Dictionary).size());
        var selector = :length as L.Object;
        System.println("x" has (selector as L.Symbol));
        var callback = "abc".method(:length) as L.Object;
        var result = (callback as L.Method).invoke();
        if (result != null) { System.println(result.toString()); }
        var maybe = 3 as L.Number or Null;
        if (maybe != null) { System.println(maybe + 2); }
        System.println(-(3 as L.Object as L.Number));
    }
}
