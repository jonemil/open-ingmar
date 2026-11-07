using Toybox.Application;
using Toybox.Lang;
using Toybox.System;

function mark(n as Lang.Number) as Lang.Number { System.println(n); return n; }
class PlainRoot {
    static var value as Lang.Number = 11;
    static function readRoot() as Lang.Number { return value; }
    function initialize() {}
}
class PlainLeaf extends PlainRoot {
    static var value as Lang.Number = 22;
    static function readLeaf() as Lang.Number { return value; }
    function initialize() {}
}
class ParentExprRoot {
    static var value as Lang.Number = mark(11);
    function initialize() {}
}
class ParentExprLeaf extends ParentExprRoot {
    static var value as Lang.Number = 22;
    function initialize() {}
}
class BothExprRoot {
    static var value as Lang.Number = mark(31);
    function initialize() {}
}
class BothExprLeaf extends BothExprRoot {
    static var value as Lang.Number = mark(32);
    function initialize() {}
}
class TypedRoot {
    static var value as Lang.Number = 7;
    static function readRoot() as Lang.Number { return value; }
    function initialize() {}
}
class TypedLeaf extends TypedRoot {
    static var value as Lang.String = "leaf";
    static function readLeaf() as Lang.String { return value; }
    function initialize() {}
}
class AccessRoot {
    static var value as Lang.Number = 11;
    function initialize() {}
}
class AccessLeaf extends AccessRoot {
    protected static var value as Lang.Number = 22;
    static function readLeaf() as Lang.Number { return value; }
    function initialize() {}
}
class PrivateRoot {
    private static var value as Lang.Number = 11;
    static function readRoot() as Lang.Number { return value; }
    function initialize() {}
}
class PrivateLeaf extends PrivateRoot {
    static var value as Lang.String = "leaf";
    function initialize() {}
}
class ReadonlyLeafRoot {
    static var value as Lang.Number = 11;
    static function writeRoot() as Void { value = 30; }
    function initialize() {}
}
class ReadonlyLeaf extends ReadonlyLeafRoot {
    static const value as Lang.Number = 22;
    function initialize() {}
}
class ReadonlyRoot {
    static const value as Lang.Number = 11;
    function initialize() {}
}
class MutableLeaf extends ReadonlyRoot {
    static var value as Lang.Number = 22;
    function initialize() {}
}
class ArrayRoot {
    static const items as Lang.Array<Lang.Number> = [1];
    function initialize() {}
}
class ArrayLeaf extends ArrayRoot {
    static const items as Lang.Array<Lang.Number> = [2];
    function initialize() {}
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        System.println(PlainLeaf.value);
        System.println(PlainRoot.value);
        System.println(PlainLeaf.readLeaf());
        System.println(PlainRoot.readRoot());
        PlainLeaf.value = 25;
        PlainRoot.value = 15;
        System.println(PlainLeaf.readLeaf());
        System.println(PlainRoot.readRoot());
        System.println(ParentExprLeaf.value);
        System.println(ParentExprRoot.value);
        System.println(ParentExprLeaf.value);
        System.println(BothExprLeaf.value);
        System.println(BothExprRoot.value);
        System.println(BothExprLeaf.value);
        System.println(TypedLeaf.readLeaf());
        System.println(TypedRoot.readRoot());
        TypedLeaf.value = "changed";
        System.println(TypedLeaf.readLeaf());
        System.println(TypedRoot.readRoot());
        System.println(AccessLeaf.readLeaf());
        System.println(AccessRoot.value);
        System.println(PrivateLeaf.value);
        System.println(PrivateRoot.readRoot());
        ReadonlyLeafRoot.writeRoot();
        System.println(ReadonlyLeafRoot.value);
        System.println(ReadonlyLeaf.value);
        MutableLeaf.value = 33;
        System.println(MutableLeaf.value);
        System.println(ReadonlyRoot.value);
        ArrayLeaf.items.add(3);
        System.println(ArrayLeaf.items.size());
        System.println(ArrayRoot.items.size());
        System.exit();
    }
}
