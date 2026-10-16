using Toybox.Application;
using Toybox.System;
module Flags {
    function make() as Toybox.Lang.Array<Toybox.Lang.Boolean> { return [true, false, true]; }
    function empty() as Toybox.Lang.Array<Toybox.Lang.Boolean> { return []; }
    function mark(b as Toybox.Lang.Boolean) as Toybox.Lang.Boolean { System.println(b); return b; }
    function computed(b as Toybox.Lang.Boolean) as Toybox.Lang.Array<Toybox.Lang.Boolean> {
        return [b, !b, b ? Flags.mark(true) : Flags.mark(false), false || b];
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        var a = Flags.make();
        var shared = a;
        a[0] = false;
        System.println(shared[0]);
        System.println(Flags.make()[0]);
        System.println(a.toString());
        System.println(a.indexOf(true));
        System.println(a.indexOf(false));
        a.add(true).addAll([false, true]);
        System.println(a.toString());
        System.println(a.remove(false));
        System.println(a.removeAll(true));
        System.println(a.toString());
        System.println(a.reverse().toString());
        System.println(a.slice(0, 1).toString());
        if (!a[0]) { System.println("false slot"); }
        a[0] = Flags.mark(true);
        System.println(a[0] && true);
        a = [];
        System.println(a.size());
        System.println(Flags.empty().add(true).toString());
        System.println(Flags.computed(true).toString());
        System.println(Flags.computed(false).toString());
        System.println([Flags.mark(true), Flags.mark(false)].toString());
        System.exit();
    }
}
