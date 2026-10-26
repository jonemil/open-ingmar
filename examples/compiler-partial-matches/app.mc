using Toybox.Application;
using Toybox.Lang;
using Toybox.System;

module Partial {
    function number(x as Lang.Number or Lang.String) as Lang.Number { return x; }
    function take(x as Lang.Number) as Lang.Number { return x; }
    function call(x as Lang.Number or Lang.String) as Lang.Number { return take(x); }
    function array(x as Lang.Number or Lang.String) as Lang.Array<Lang.Number> {
        return [x];
    }
    function dictionary(x as Lang.Number or Lang.String) as Lang.Dictionary<Lang.Number, Lang.Number> {
        return {1 => x};
    }
    function record(x as Lang.Number or Lang.String) as {:value as Lang.Number} {
        return {:value => x};
    }
    function slice(x as Lang.Number or Lang.String) as Lang.String {
        return "abc".substring(x, 3);
    }
    function assign(n as Lang.Number, x as Lang.Number or Lang.String) as Lang.Number {
        n = x;
        return n;
    }
}

class Holder {
    var value = 0;
    function initialize(x as Lang.Number or Lang.String) { self.value = x; }
    function read() as Lang.Number { return self.value; }
}

class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(Partial.number(41));
        System.println(Partial.call(42));
        System.println(new Holder(43).read());
        System.println(Partial.array(44)[0]);
        var stored = Partial.dictionary(45)[1];
        if (stored != null) { System.println(stored); }
        var field = Partial.record(46)[:value];
        if (field != null) { System.println(field); }
        System.println(Partial.slice(0));
        System.println(Partial.assign(0, 47));
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
