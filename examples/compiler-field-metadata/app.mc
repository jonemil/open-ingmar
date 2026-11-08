using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
function computedText() as Lang.String { System.println("allocate"); return "computed"; }
class Defaults {
    var text as Lang.String = "literal";
    const frozen as Lang.String = "fixed";
    var flag as Lang.Boolean = true;
    var ch as Lang.Char = 'Q';
    var flt as Lang.Float = 1.5f;
    var lng as Lang.Long = 4294967297l;
    var dbl as Lang.Double = 2.5d;
    var grouped as Lang.String = ("grouped");
    var groupedNumber as Lang.Number = (7);
    const groupedConst as Lang.String = ("grouped const");
    var expr as Lang.String = computedText();
    function initialize() {}
    function classLong() as Lang.Long { return Defaults.lng; }
    function classDouble() as Lang.Double { return Defaults.dbl; }
    function classGrouped() as Lang.String { return Defaults.grouped; }
    function classGroupedNumber() as Lang.Number { return Defaults.groupedNumber; }
    function classExpr() as Lang.String { return Defaults.expr; }
    function classFlag() as Lang.Boolean { return Defaults.flag; }
    function classChar() as Lang.Char { return Defaults.ch; }
    function classFloat() as Lang.Float { return Defaults.flt; }
    function classTextIsNull() as Lang.Boolean { return Defaults.text == null; }
}
class Parent {
    var text as Lang.String = computedText();
    function initialize() {}
}
class Child extends Parent {
    var text as Lang.String = "child";
    function initialize() {}
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        var a = new Defaults();
        System.println(a.text);
        System.println(a.frozen);
        System.println(a.flag);
        System.println(a.ch == 'Q');
        System.println(a.flt == 1.5f);
        System.println(a.lng == 4294967297l);
        System.println(a.dbl == 2.5d);
        System.println(a.expr);
        System.println(a.grouped);
        System.println(a.groupedNumber);
        System.println(a.groupedConst);
        System.println(a.classGrouped());
        System.println(a.classGroupedNumber());
        System.println(a.classExpr());
        System.println(a.classFlag());
        System.println(a.classChar() == 'Q');
        System.println(a.classFloat() == 1.5f);
        a.text = "changed";
        var b = new Defaults();
        System.println(a.text);
        System.println(b.text);
        var c = new Child();
        System.println(c.text);
        System.exit();
    }
}
