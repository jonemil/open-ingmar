using Toybox.Application;
using Toybox.System;
module Strings {
    function constants() as Toybox.Lang.Array<Toybox.Lang.String> {
        return ["one", "two", "one", "é", ""];
    }
    function empty() as Toybox.Lang.Array<Toybox.Lang.String> { return []; }
    function mark(s as Toybox.Lang.String) as Toybox.Lang.String {
        System.println(s); return s;
    }
    function mutate(a as Toybox.Lang.Array<Toybox.Lang.String>) as Toybox.Lang.String {
        System.println("mutate"); a[0] = "new"; return "new";
    }
    function receiver(a as Toybox.Lang.Array<Toybox.Lang.String>)
        as Toybox.Lang.Array<Toybox.Lang.String> {
        System.println("receiver"); return a;
    }
    function index() { System.println("index"); return 0; }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        var first = Strings.constants();
        var snapshot = first.toString();
        first[0] = "changed";
        var second = Strings.constants();
        System.println(first[0]);
        System.println(second[0]);
        System.println(snapshot);
        System.println(second.size());
        System.println(second.indexOf("one"));
        System.println(second[3].length());
        var a = ["a", "b"];
        var shared = a;
        a[1] = "B";
        System.println(shared[1]);
        a.add("c").addAll(["d", "c"]);
        System.println(a.toString());
        System.println(a.remove("c"));
        System.println(a.removeAll("c"));
        System.println(a.indexOf("B"));
        System.println(a.indexOf("absent"));
        System.println(a.reverse().toString());
        System.println(a.slice(null, 2).toString());
        System.println(a.toString());
        a = [];
        System.println(a.toString());
        System.println(Strings.empty().add("x").toString());
        System.println([Strings.mark("left"), Strings.mark("right")].toString());
        System.println([true ? Strings.mark("yes") : Strings.mark("skip"), "tail"].toString());
        var nullable = "abc".substring(0, 1);
        if (nullable != null) { System.println([nullable, "z"].toString()); }
        var current = ["old"];
        System.println([current[0], Strings.mutate(current)].toString());
        Strings.receiver(current)[Strings.index()] = Strings.mark("done");
        System.println(current[0]);
        System.exit();
    }
}
