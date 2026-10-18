using Toybox.Application;
using Toybox.System;
using Toybox.WatchUi;
module Callbacks {
    function bind(o as Toybox.Lang.Object, s as Toybox.Lang.Symbol) as Toybox.Lang.Method { return o.method(s); }
    function show(m as Toybox.Lang.Method, n as Toybox.Lang.Number) as Void {
        var value = m.invoke(n);
        if (value != null) { System.println(value.toString()); }
    }
}
class CallbackView extends WatchUi.View {
    var count = 0;
    function initialize() { View.initialize(); }
    function add(n as Toybox.Lang.Number) as Toybox.Lang.Number {
        self.count = self.count + n;
        return self.count;
    }
    function bind(s as Toybox.Lang.Symbol) as Toybox.Lang.Method {
        return method(s);
    }
    function owner() as Toybox.Lang.Object { return self; }
}
class MinimalConsoleApp extends Application.AppBase {
    var count = 10;
    function initialize() { AppBase.initialize(); }
    function add(n as Toybox.Lang.Number) as Toybox.Lang.Number {
        self.count = self.count + n;
        return self.count;
    }
    function onStart(state) {
        var view = new CallbackView();
        var direct = view.method(:add);
        var bound = view.bind(:add);
        var erased = Callbacks.bind(view.owner(), :add);
        Callbacks.show(direct, 2);
        Callbacks.show(bound, 3);
        Callbacks.show(erased, 4);
        view = new CallbackView();
        Callbacks.show(direct, 1);
        System.println(view.add(0));
        var own = method(:add);
        Callbacks.show(own, 5);
        method(:add);
        System.exit();
    }
}
