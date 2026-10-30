using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
module Trace {
    function number(value as Lang.Number) as Lang.Number {
        System.println(value);
        return value;
    }
}
class Token {
    function value() as Lang.Number { return 41; }
}
class Box {
    var value = 1;
    var selector = :start;
    var text = "before";
    var computed = Trace.number(10);
    var items = [1,2];
    var object = new Token();
    function change() {
        self.value = "changed";
        System.println(self.value.length());
        self.selector = 42;
        System.println(self.selector);
        self.text = [3,4];
        System.println(self.text.size());
        self.items = "new";
        System.println(self.items.length());
        self.object = "object";
        System.println(self.object.length());
    }
}
class MinimalConsoleApp extends Application.AppBase {
    var marker = :app;
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println("begin");
        var a = new Box();
        var b = new Box();
        System.println(a.value == 1);
        System.println(a.selector == :start);
        System.println(a.computed);
        var token = a.object;
        if (token instanceof Token) { System.println(token.value()); }
        a.change();
        System.println(b.value == 1);
        System.println(b.selector == :start);
        System.println(b.items.size());
        System.println(a.object.length());
        a.value = [8];
        var values = a.value;
        if (values instanceof Lang.Array) { System.println(values[0]); }
        System.println(self.marker.toString());
        System.exit();
    }
}
