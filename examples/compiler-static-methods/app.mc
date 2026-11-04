using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
module Trace {
    var LOG as Lang.Number = 0;
    function mark(n as Lang.Number) as Lang.Number { LOG = LOG * 10 + n; return n; }
}
class Counter {
    static var VALUE as Lang.Number = Trace.mark(2);
    static var FROM_HELPER as Lang.Number = seed();
    static const OWNER = self;
    static function seed() as Lang.Number { return Trace.mark(3); }
    static function add(n as Lang.Number) as Lang.Number { VALUE += n; return VALUE; }
    static function chain(n as Lang.Number) as Lang.Number { return self.add(n) + add(1); }
    static function owner() { return self; }
    static function callback() as Lang.Method { return new Lang.Method(self, :add); }
    var own as Lang.Number = Trace.mark(4);
    function initialize() {}
    function bump() as Lang.Number { return add(1); }
}
class Utility {
    static function twice(n as Lang.Number) as Lang.Number { return n * 2; }
}
module Nested {
    var BASE as Lang.Number = 10;
    function base() as Lang.Number { return BASE; }
    class Settings {
        static function compute(n as Lang.Number) as Lang.Number { return base() + n; }
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        System.println(Trace.LOG);
        var first = new Counter();
        System.println(Trace.LOG);
        System.println(first.add(4));
        var second = new Counter();
        System.println(second.bump());
        System.println(first.chain(2));
        System.println(first.owner() == Counter);
        System.println(first.owner() == second.owner());
        System.println(first.owner() == first);
        System.println(first.owner() == Counter.OWNER);
        System.println(Counter.OWNER == Counter);
        var bound = first.method(:add);
        System.println(bound.invoke(5));
        var explicit = first.callback();
        System.println(explicit.invoke(2) as Lang.Number);
        var stored = new Lang.Method(Counter.OWNER, :add);
        System.println(stored.invoke(1) as Lang.Number);
        first = new Counter();
        System.println(bound.invoke(1));
        System.println(Trace.LOG);
        var utility = new Utility();
        System.println(utility.twice(4));
        var settings = new Nested.Settings();
        System.println(settings.compute(8));
        System.exit();
    }
}
