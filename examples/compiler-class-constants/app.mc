using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
module Trace {
    function argument() as Lang.Number { System.println(3); return 4; }
    function build() as Lang.Array<Lang.Number> { System.println(1); return [7]; }
}
class Token {
    function value() as Lang.Number { return 41; }
}
class Box {
    const TEXT = "member";
    const BIG = -2147483649l;
    const WIDE = 2.5d;
    const ITEMS = [3];
    const ALIAS = ITEMS;
    const VALUES = {"value"=>4};
    const TOKEN = new Token();
    const COMPUTED = Trace.build();
    const EMPTY = null;
    function initialize() { System.println(2); }
}
class ArgBox {
    const ITEMS = Trace.build();
    function initialize(value as Lang.Number) { System.println(value); }
}
class One { const VALUE = 1; }
class Two { const VALUE = 2; }
function value(box as One or Two) as Lang.Number { return box.VALUE; }
class MinimalConsoleApp extends Application.AppBase {
    const TEXT = "application";
    const ITEMS = [5];
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        var a = new Box();
        var b = new Box();
        a.ITEMS[0] = 9;
        System.println(a.ITEMS[0]);
        System.println(b.ITEMS[0]);
        System.println(a.ALIAS[0]);
        a.ITEMS.add(10);
        System.println(a.ALIAS.size());
        System.println(a.TEXT == a.TEXT);
        System.println(a.TEXT == b.TEXT);
        System.println(a.BIG == -2147483649l);
        System.println(a.WIDE == 2.5d);
        var map = a.VALUES;
        map["value"] = 42;
        var saved = a.VALUES["value"];
        if (saved != null) { System.println(saved); }
        System.println(a.TOKEN.value());
        a.initialize();
        System.println(a.ITEMS[0]);
        System.println(a.COMPUTED[0]);
        System.println(a.EMPTY == null);
        var weak = a.ITEMS.weak();
        System.println(weak.get() != null);
        System.println(ITEMS[0]);
        System.println(TEXT == TEXT);
        var c = new ArgBox(Trace.argument());
        System.println(c.ITEMS[0]);
        System.println(value(new One()));
        System.println(value(new Two()));
        var d = new ArgBox(true ? Trace.argument() : 8);
        System.println(d.ITEMS[0]);
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
