using Toybox.Application;
using Toybox.Lang;
using Toybox.System;

typedef DeclaredSlot as interface { var slot as Lang.String; };
function readDeclared(value as DeclaredSlot) as Lang.String { return value.slot; }
function readMutable(value as MutableInstanceRoot) as Lang.Number { return value.slot; }
function readBase(value as InstanceRoot) as Lang.Number { return value.slot; }
function textDefault() as Lang.String { System.println("static init"); return "leaf"; }
class InstanceRoot {
    var slot as Lang.Number = 11;
    function initialize() {}
}
class StaticLeaf extends InstanceRoot {
    static var slot as Lang.String = textDefault();
    var derived as Lang.Number = slot + 1;
    function initialize() {}
    function read() as Lang.Number { return slot; }
    function setInstance() as Void { slot = 13; }
    function readQualified() as Lang.Number { return StaticLeaf.slot; }
    static function readStatic() as Lang.String { return slot; }
    static function setStatic() as Void { slot = "changed"; }
}
class PrivateStaticLeaf extends InstanceRoot {
    private static var slot as Lang.String = "private";
    function initialize() {}
    function read() as Lang.Number { return slot; }
    function readQualified() as Lang.Number { return PrivateStaticLeaf.slot; }
    static function readStatic() as Lang.String { return slot; }
}
class SharedRoot {
    static var slot as Lang.Number = 21;
    function initialize() {}
}
class InstanceLeaf extends SharedRoot {
    var slot as Lang.String = "child";
    function initialize() {}
    function read() as Lang.String { return slot; }
}
class MutableInstanceRoot {
    var slot as Lang.Number = 31;
    function initialize() {}
}
class ConstStaticLeaf extends MutableInstanceRoot {
    static const slot as Lang.String = "frozen";
    function initialize() {}
    static function readStatic() as Lang.String { return slot; }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        var a = new StaticLeaf();
        System.println(a.slot);
        System.println(a.derived);
        System.println(a.read());
        System.println(readBase(a));
        a.setInstance();
        System.println(a.slot);
        System.println(StaticLeaf.readStatic());
        System.println(a.slot);
        System.println(a.readQualified());
        StaticLeaf.setStatic();
        System.println(a.readQualified());
        System.println(readDeclared(a));
        var p = new PrivateStaticLeaf();
        System.println(p.slot);
        System.println(p.read());
        System.println(PrivateStaticLeaf.readStatic());
        System.println(p.readQualified());
        var b = new InstanceLeaf();
        System.println(b.slot);
        System.println(b.read());
        SharedRoot.slot = 25;
        System.println(b.slot);
        System.println(SharedRoot.slot);
        b.slot = "updated";
        System.println(b.slot);
        System.println(SharedRoot.slot);
        var c = new ConstStaticLeaf();
        c.slot = 32;
        System.println(c.slot);
        System.println(readMutable(c));
        System.println(ConstStaticLeaf.readStatic());
        System.exit();
    }
}
