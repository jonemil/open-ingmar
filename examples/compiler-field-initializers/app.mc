using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
module Trace {
    function number(value as Lang.Number) as Lang.Number {
        System.println(value);
        return value;
    }
    function array() as Lang.Array<Lang.Number> {
        System.println(30);
        return [2];
    }
}
class MinimalConsoleApp extends Application.AppBase {
    var message as Lang.String = "application";
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(self.message);
        var a = new Box();
        var b = new Box();
        System.println(a.snapshot);
        System.println(a.text);
        System.println(a.flag);
        System.println(a.letter == 'Q');
        System.println(a.fraction == 1.5f);
        System.println(a.big == -2147483649l);
        System.println(a.wide == 2.5d);
        System.println(a.optional == null);
        System.println(a.selected);
        a.items[0] = 9;
        System.println(a.items[0]);
        System.println(b.items[0]);
        a.values["value"] = 8;
        var first = a.values["value"];
        var second = b.values["value"];
        if (first != null) { System.println(first); }
        if (second != null) { System.println(second); }
        a.bytes[0] = 5;
        System.println(a.bytes[0]);
        System.println(b.bytes[0]);
        System.println(a.token.value());
        a.change();
        System.println(a.text);
        System.println(a.items.size());
        System.println(b.items.size());
        System.exit();
    }
}
