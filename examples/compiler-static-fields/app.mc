using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
module Trace {
    var LOG as Lang.Number = 0;
    function mark(n as Lang.Number) as Lang.Number { LOG = LOG * 10 + n; return n; }
}
class Cold {
    static var VALUE as Lang.Number = Trace.mark(2);
    static var DATA as Lang.Array<Lang.Number> = [3];
    static var DYNAMIC = 2;
    var own as Lang.Number = Trace.mark(4);
    function initialize() {}
    function bump() as Lang.Number { VALUE++; return VALUE; }
}
class Warm {
    static var VALUE as Lang.Number = Trace.mark(6);
}
module Nested {
    var BASE as Lang.Number = 10;
    class Settings {
        static var COUNT as Lang.Number = BASE;
        static var TEXT as Lang.String or Null;
    }
}
function overwriteCold() as Lang.Number {
    Cold.VALUE = 50;
    return Trace.mark(3);
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        System.println(Trace.LOG);
        Cold.VALUE = Trace.mark(1);
        System.println(Trace.LOG);
        System.println(Cold.VALUE);
        Warm.VALUE += Trace.mark(7);
        System.println(Warm.VALUE);
        System.println(Trace.LOG);
        Cold.VALUE += overwriteCold();
        System.println(Cold.VALUE);
        Cold.VALUE++;
        var first = new Cold();
        var second = new Cold();
        System.println(first.bump());
        second.VALUE = 10;
        System.println(Cold.VALUE);
        System.println(Trace.LOG);
        var weak = Cold.DATA.weak();
        Cold.DATA = true ? [9] : [8];
        System.println(weak.stillAlive());
        System.println(Cold.DATA[0]);
        Cold.DYNAMIC = "changed";
        System.println(Cold.DYNAMIC);
        Nested.Settings.COUNT += true ? 2 : 3;
        System.println(Nested.Settings.COUNT);
        System.println(Nested.Settings.TEXT == null);
        Nested.Settings.TEXT = "set";
        System.println(Nested.Settings.TEXT as Lang.String);
        System.exit();
    }
}
