using Toybox.Application;
using Toybox.System;
module Dictionaries {
    function make() as Toybox.Lang.Dictionary { System.println("make"); return {}; }
    function size(d as Toybox.Lang.Dictionary) { System.println("size"); return d.size(); }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        var d = {};
        System.println(d.size()); System.println(d.isEmpty());
        var shared = d;
        System.println(shared.size());
        System.println(Dictionaries.size(Dictionaries.make()));
        System.println(Dictionaries.make().isEmpty());
        var n = 0;
        while (n < 2) { d = {}; System.println(d.isEmpty()); n++; }
        if (state != null) { System.println(state.size()); }
        System.exit();
    }
}
