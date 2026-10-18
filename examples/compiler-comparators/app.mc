using Toybox.Application;
using Toybox.System;
module Comparators {
    function array(a as Toybox.Lang.Array<Toybox.Lang.String>) as Toybox.Lang.Array<Toybox.Lang.String> {
        System.println("array");
        return a;
    }
    function comparator(c as Ordering) as Ordering {
        System.println("comparator");
        return c;
    }
    function sort(a as Toybox.Lang.Array<Toybox.Lang.String>, c as Ordering) as Void { a.sort(c); }
    function empty() as Toybox.Lang.Array<Toybox.Lang.String> { return []; }
    function run() as Void {
        InterfaceCalls.run();
        var text = new TextOrder();
        System.println(text.compareTo("a") > 0);
        System.println(text.compareTo("b") == 0);
        System.println(text.compareTo("c") < 0);
        var ascending = new Ordering(1);
        var descending = new Ordering(-1);
        var a = ["bb", "a", "aa", "b", "a"];
        var shared = a;
        Comparators.array(a).sort(Comparators.comparator(descending));
        System.println(shared.toString());
        System.println(descending.calls() > 0);
        Comparators.sort(a, ascending);
        System.println(a.toString());
        System.println(ascending.calls() > 0);
        var numbers = [2, 10, 1];
        numbers.sort(descending);
        System.println(numbers.toString());
        var empty = Comparators.empty();
        empty.sort(ascending);
        System.println(empty.size());
        var singleton = ["only"];
        singleton.sort(descending);
        System.println(singleton[0]);
    }
}
module InterfaceCalls {
    function comparable(value as Toybox.Lang.Comparable) as Toybox.Lang.Comparable { return value; }
    function create() as Toybox.Lang.Comparable { return new TextOrder(); }
    function comparator(value as Ordering) as Toybox.Lang.Comparator { return value; }
    function forward(value as Toybox.Lang.Comparator) as Toybox.Lang.Comparator { return value; }
    function receiver(value as Toybox.Lang.Comparable) as Toybox.Lang.Comparable {
        System.println("interface-receiver"); return value;
    }
    function argument() as Toybox.Lang.Object { System.println("interface-argument"); return "a"; }
    function run() as Void {
        var text = InterfaceCalls.create();
        var shared = InterfaceCalls.comparable(text);
        System.println(shared.compareTo("b") == 0);
        System.println(InterfaceCalls.receiver(text).compareTo(InterfaceCalls.argument()) > 0);
        shared = InterfaceCalls.comparable(new TextOrder());
        System.println(shared.compareTo("c") < 0);
        var order = InterfaceCalls.comparator(new Ordering(-1));
        order = InterfaceCalls.forward(order);
        System.println(order.compare("a", "b") > 0);
        var values = ["a", "c", "b"];
        values.sort(order);
        System.println(values.toString());
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Comparators.run(); System.exit(); }
}
