using Toybox.Application;
using Toybox.Lang;
using Toybox.System;

function mark(n as Lang.Number) as Lang.Number { System.println(n); return n; }
class Root {
    static var value as Lang.Number = mark(10);
    static const items as Lang.Array<Lang.Number> = [1];
    static function owner() { return self; }
    static function read() as Lang.Number { return value; }
    static function readQualified() as Lang.Number { return Root.value; }
    static function bump() { value += 1; }
    function initialize() {}
}
class Middle extends Root {
    static var middle as Lang.Number = mark(20);
    function initialize() {}
}
class Leaf extends Middle {
    static var child as Lang.Number = mark(30);
    function initialize() {}
}
class Empty extends Root {
    function initialize() {}
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        System.println(Leaf.owner() == Root.owner());
        System.println(Leaf.value);
        Root.bump();
        System.println(Leaf.value);
        Leaf.items.add(2);
        System.println(Root.items.size());
        System.println(Leaf.child);
        System.println(Middle.middle);
        System.println(Empty.owner() == Root.owner());
        System.println(Empty.value);
        var instance = new Leaf();
        System.println(instance.owner() == instance);
        System.println(instance.value);
        System.println(instance.read());
        instance.bump();
        System.println(Root.value);
        System.println(Leaf.readQualified());
        var empty = new Empty();
        System.println(empty.value);
        System.exit();
    }
}
