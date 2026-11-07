using Toybox.Application;
using Toybox.Lang;
using Toybox.System;

function mark(n as Lang.Number) as Lang.Number { System.println(n); return n; }
class LiteralRoot {
    var value as Lang.Number = 11;
    function initialize() {}
    function readRoot() as Lang.Number { return value; }
    function writeRoot() { value = 30; }
}
class LiteralLeaf extends LiteralRoot {
    var value as Lang.Number = 22;
    function initialize() {}
}
function readLiteral(x as LiteralRoot) as Lang.Number { return x.value; }
class ParentExprRoot {
    var value as Lang.Number = mark(11);
    function initialize() {}
    function readRoot() as Lang.Number { return value; }
}
class ParentExprLeaf extends ParentExprRoot {
    var value as Lang.Number = 22;
    function initialize() {}
}
class ChildExprRoot {
    var value as Lang.Number = 11;
    function initialize() {}
    function readRoot() as Lang.Number { return value; }
}
class ChildExprLeaf extends ChildExprRoot {
    var value as Lang.Number = mark(22);
    function initialize() {}
}
class BothExprRoot {
    var value as Lang.Number = mark(31);
    function initialize() {}
    function readRoot() as Lang.Number { return value; }
}
class BothExprLeaf extends BothExprRoot {
    var value as Lang.Number = mark(32);
    function initialize() {}
}
class TypedRoot {
    var value as Lang.Number = 11;
    function initialize() {}
    function readRoot() as Lang.Number { return value; }
    function writeRoot() { value = 30; }
}
class TypedLeaf extends TypedRoot {
    var value as Lang.String = "leaf";
    function initialize() {}
    function readLeaf() as Lang.String { return value; }
}
function readTyped(x as TypedRoot) as Lang.Number { return x.value; }
class AccessRoot {
    var value as Lang.Number = 11;
    function initialize() {}
    function readRoot() as Lang.Number { return value; }
    function writeRoot() { value = 30; }
}
class AccessLeaf extends AccessRoot {
    protected var value as Lang.Number = 22;
    function initialize() {}
    function readLeaf() as Lang.Number { return value; }
}
function readAccess(x as AccessRoot) as Lang.Number { return x.value; }
class PrivateRoot {
    var value as Lang.Number = 11;
    function initialize() {}
    function readRoot() as Lang.Number { return value; }
    function writeRoot() { value = 30; }
}
class PrivateLeaf extends PrivateRoot {
    private var value as Lang.Number = 22;
    function initialize() {}
    function readLeaf() as Lang.Number { return value; }
}
class ConstLeafRoot {
    var value as Lang.Number = mark(41);
    function initialize() {}
    function readRoot() as Lang.Number { return value; }
}
class ConstLeaf extends ConstLeafRoot {
    const value as Lang.Number = 42;
    function initialize() {}
}
function writeConstThroughParent(x as ConstLeafRoot) { x.value = 50; }
class ConstRoot {
    const value as Lang.Number = mark(51);
    function initialize() {}
    function readRoot() as Lang.Number { return value; }
}
class MutableLeaf extends ConstRoot {
    var value as Lang.Number = 52;
    function initialize() {}
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        var literal = new LiteralLeaf();
        System.println(literal.value);
        System.println(readLiteral(literal));
        System.println(literal.readRoot());
        literal.writeRoot();
        System.println(literal.value);
        var parentExpr = new ParentExprLeaf();
        System.println(parentExpr.value);
        System.println(parentExpr.readRoot());
        var childExpr = new ChildExprLeaf();
        System.println(childExpr.value);
        System.println(childExpr.readRoot());
        var bothExpr = new BothExprLeaf();
        System.println(bothExpr.value);
        System.println(bothExpr.readRoot());
        var typed = new TypedLeaf();
        System.println(typed.value);
        System.println(readTyped(typed));
        System.println(typed.readRoot());
        typed.writeRoot();
        System.println(typed.readLeaf());
        var access = new AccessLeaf();
        System.println(readAccess(access));
        System.println(access.readRoot());
        System.println(access.readLeaf());
        access.writeRoot();
        System.println(access.readLeaf());
        System.println(readAccess(access));
        var privateField = new PrivateLeaf();
        System.println(privateField.readRoot());
        System.println(privateField.readLeaf());
        privateField.writeRoot();
        System.println(privateField.readLeaf());
        System.println(privateField.readRoot());
        var constLeaf = new ConstLeaf();
        System.println(constLeaf.value);
        System.println(constLeaf.readRoot());
        writeConstThroughParent(constLeaf);
        System.println(constLeaf.value);
        var mutableLeaf = new MutableLeaf();
        System.println(mutableLeaf.value);
        mutableLeaf.value = 53;
        System.println(mutableLeaf.readRoot());
        System.exit();
    }
}
