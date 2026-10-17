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
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Comparators.run(); System.exit(); }
}
