using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
const TEXT = "shared";
const BIG = -2147483649l;
const WIDE = 2.5d;
const ITEMS = [1];
const ALIAS = ITEMS;
const VALUES = {"value"=>4};
const TOKEN = new Token();
const COMPUTED = Trace.build(6);
const BYTES = [1,2]b;
const WEAK = ITEMS.weak();
const BOUND = TOKEN.method(:value);
module Trace {
    function build(n as Lang.Number) as Lang.Array<Lang.Number> {
        System.println(n);
        return [n];
    }
}
module M {
    const TEXT = "module";
    const ITEMS = Trace.build(2);
    const ALIAS = ITEMS;
    function get() as Lang.Array<Lang.Number> { return self.ITEMS; }
    function name() as Lang.String { return TEXT; }
}
class Token {
    function value() as Lang.Number { return 41; }
}
class MinimalConsoleApp extends Application.AppBase {
    const START = ITEMS;
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(4);
        System.println(COMPUTED[0]);
        System.println(TEXT == TEXT);
        System.println(BIG == -2147483649l);
        System.println(WIDE == 2.5d);
        ITEMS[0] = 7;
        ITEMS.add(9);
        System.println(ALIAS[0]);
        System.println(ALIAS.size());
        System.println(START[0]);
        System.println(M.ITEMS[0]);
        var other = M.get();
        other[0] = 8;
        System.println(M.ALIAS[0]);
        System.println(M.get()[0]);
        System.println(M.TEXT == M.name());
        var map = VALUES;
        map["value"] = 42;
        var saved = VALUES["value"];
        if (saved != null) { System.println(saved); }
        System.println(TOKEN.value());
        var weak = ITEMS.weak();
        System.println(weak.get() != null);
        BYTES[0] = 9;
        System.println(BYTES[0]);
        System.println(WEAK.get() != null);
        System.println(BOUND.invoke());
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
