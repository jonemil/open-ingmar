using Toybox.Application;
using Toybox.System;
module Sorting {
    function receiver(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Array<Toybox.Lang.Number> {
        System.println("receiver");
        return a;
    }
    function empty() as Toybox.Lang.Array<Toybox.Lang.Number> { return []; }
    function run() as Void {
        var a = [3, -1, 2, 2];
        var shared = a;
        var copy = a.slice(0, null);
        Sorting.receiver(a).sort(null);
        System.println(shared.toString());
        System.println(copy.toString());
        a.sort(null);
        System.println(a.toString());
        var longs = [4294967297l, -1l, 0l];
        longs.sort(null);
        System.println(longs[0] == -1l);
        System.println(longs[2] == 4294967297l);
        var floats = [3.25, -1.5, 0.0];
        floats.sort(null);
        System.println(floats[0] == -1.5);
        System.println(floats[2] == 3.25);
        var doubles = [16777217.0d, -1.5d, 0.0d];
        doubles.sort(null);
        System.println(doubles[0] == -1.5d);
        System.println(doubles[2] == 16777217.0d);
        var strings = ["bb", "a", "aa", "b"];
        strings.sort(null);
        System.println(strings.toString());
        var booleans = [true, false, true, false];
        booleans.sort(null);
        System.println(booleans[0]);
        System.println(booleans[3]);
        var chars = ['z', 'a', 'b'];
        chars.sort(null);
        System.println(chars[0]);
        System.println(chars[2]);
        var empty = Sorting.empty();
        empty.sort(null);
        System.println(empty.size());
        var singleton = [7];
        singleton.sort(null);
        System.println(singleton[0]);
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Sorting.run(); System.exit(); }
}
