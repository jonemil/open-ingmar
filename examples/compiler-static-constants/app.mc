using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
module Trace {
    var LOG as Lang.Number = 0;
    function mark(n as Lang.Number) as Lang.Number {
        LOG = LOG * 10 + n;
        return n;
    }
}
module Nested {
    var BASE as Lang.Number = 10;
    function select() as Lang.Number { return BASE; }
    class Settings {
        static const LOCAL as Lang.Number = select();
    }
}
class Holder {
    static const DEFAULT as Lang.Number = 7;
    static const MESSAGE as Lang.String = "shared";
    static const ITEMS as Lang.Array<Lang.Number> = [Trace.mark(2)];
    static const COPY as Lang.Number = Trace.mark(3);
    static const CHOICE as Lang.Number = true ? Trace.mark(5) : Trace.mark(6);
    static const CALLBACK as Lang.Method = new Lang.Method(-7, :abs);
    var own as Lang.Number = Trace.mark(4);
    function initialize() {}
    function size() as Lang.Number { return ITEMS.size(); }
    function literal() as Lang.Number { return self.DEFAULT; }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        System.println(Trace.LOG);
        var klass = Holder;
        System.println(Trace.LOG);
        System.println(Holder.DEFAULT);
        System.println(Trace.LOG);
        System.println(Holder.MESSAGE);
        System.println(Holder.ITEMS.size());
        Holder.ITEMS.add(9);
        var first = new Holder();
        var second = new Holder();
        System.println(first.size());
        System.println(second.literal());
        System.println(Trace.LOG);
        System.println(Holder.CALLBACK.invoke() as Lang.Number);
        System.println(Nested.Settings.LOCAL);
        System.exit();
    }
}
