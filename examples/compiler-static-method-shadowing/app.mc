using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
function mark(value as Lang.Number) as Lang.Number { System.println(value); return value; }
class Root {
    function initialize() {}
    function pick(n as Lang.Number) as Lang.Number { return 11; }
    function forward() as Lang.Number { return pick(0); }
}
class StaticLeaf extends Root {
    static var ready as Lang.Number = mark(100);
    static function pick(n as Lang.Number) as Lang.Number { return 22; }
    function own() as Lang.Number { return pick(0); }
    function qualified() as Lang.Number { return StaticLeaf.pick(0); }
    static function stat() as Lang.Number { return pick(0); }
}
class PrivateLeaf extends Root {
    static var ready as Lang.Number = mark(200);
    private static function pick(s as Lang.String) as Lang.String { return "private"; }
    function own() as Lang.Number { return pick(0); }
    function qualified() as Lang.Number { return PrivateLeaf.pick(0); }
    static function stat() as Lang.String { return pick("x"); }
}
class SharedRoot {
    static var ready as Lang.Number = mark(300);
    function initialize() {}
    static function pick(n as Lang.Number) as Lang.Number { return 33; }
    static function forward() as Lang.Number { return pick(0); }
}
class InstanceLeaf extends SharedRoot {
    function pick(s as Lang.String) as Lang.String { return "instance"; }
    function own() as Lang.String { return pick("x"); }
}
class SharedLeaf extends SharedRoot {
    static var ready as Lang.Number = mark(400);
    static function pick(s as Lang.String) as Lang.String { return "shared"; }
    static function own() as Lang.String { return pick("x"); }
}
function throughRoot(value as Root) as Lang.Number { return value.pick(0); }
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        var a = new StaticLeaf();
        System.println(a.pick(0));
        System.println(a.own());
        System.println(a.forward());
        System.println(throughRoot(a));
        System.println(a.qualified());
        System.println(StaticLeaf.stat());
        var b = new PrivateLeaf();
        System.println(b.pick(0));
        System.println(b.own());
        System.println(b.qualified());
        System.println(PrivateLeaf.stat());
        var c = new InstanceLeaf();
        System.println(c.pick("x"));
        System.println(c.own());
        System.println(SharedRoot.pick(0));
        System.println(SharedLeaf.pick("x"));
        System.println(SharedLeaf.own());
        System.exit();
    }
}
