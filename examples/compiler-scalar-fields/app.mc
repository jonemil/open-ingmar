using Toybox.Lang;
using Toybox.System;
class Scalars {
    var number as Lang.Number;
    var longValue as Lang.Long;
    var floatValue as Lang.Float;
    var doubleValue as Lang.Double;
    var booleanValue as Lang.Boolean;
    var charValue as Lang.Char;
    var stringValue as Lang.String;
    var maybeNumber as Lang.Number or Null;
    var maybeLong as Lang.Long or Null = null;
    var maybeFloat as Lang.Float or Null;
    var maybeDouble as Lang.Double or Null = null;
    var maybeBoolean as Lang.Boolean or Null;
    var maybeChar as Lang.Char or Null = null;
    var maybeString as Lang.String or Null;
    function initialize() {
        self.number = 1; self.longValue = 2147483648l;
        self.floatValue = -0.0; self.doubleValue = 4.5d;
        self.booleanValue = true; self.charValue = 'A'; self.stringValue = "start";
    }
    function change() as Void {
        self.number++; self.longValue += 2l;
        self.floatValue = 3.5; self.floatValue += 1;
        self.doubleValue *= 2.0d; self.booleanValue = !self.booleanValue;
        self.charValue += 1; self.stringValue += "!";
    }
    function show() as Void {
        System.println(self.number); System.println(self.longValue);
        System.println(self.floatValue); System.println(self.doubleValue);
        System.println(self.booleanValue); System.println(self.charValue);
        System.println(self.stringValue);
    }
    function classes() as Void {
        System.println(self.number instanceof Lang.Number); System.println(self.longValue instanceof Lang.Long);
        System.println(self.floatValue instanceof Lang.Float); System.println(self.doubleValue instanceof Lang.Double);
        System.println(self.booleanValue instanceof Lang.Boolean); System.println(self.charValue instanceof Lang.Char);
        System.println(self.stringValue instanceof Lang.String);
    }
    function clear() as Void {
        self.maybeNumber = null; self.maybeLong = null; self.maybeFloat = null;
        self.maybeDouble = null; self.maybeBoolean = null; self.maybeChar = null;
        self.maybeString = null;
    }
    function absent() as Void {
        System.println(self.maybeNumber == null); System.println(self.maybeLong == null);
        System.println(self.maybeFloat == null); System.println(self.maybeDouble == null);
        System.println(self.maybeBoolean == null); System.println(self.maybeChar == null);
        System.println(self.maybeString == null);
    }
    function snapshots() as Void {
        self.maybeNumber = self.number; self.maybeLong = self.longValue;
        self.maybeFloat = self.floatValue; self.maybeDouble = self.doubleValue;
        self.maybeBoolean = self.booleanValue; self.maybeChar = self.charValue;
        self.maybeString = self.stringValue;
        var n = self.maybeNumber; var l = self.maybeLong; var f = self.maybeFloat;
        var d = self.maybeDouble; var b = self.maybeBoolean; var c = self.maybeChar;
        var s = self.maybeString; self.clear();
        if (n != null) {System.println(n + 1);}
        if (l != null) {System.println(l + 1l);}
        if (f != null) {System.println(f + 1.0);}
        if (d != null) {System.println(d + 1.0d);}
        if (b != null) {System.println(!b);}
        if (c != null) {System.println(c.toNumber());}
        if (s != null) {System.println(s.toUpper());}
    }
}
class MinimalConsoleApp extends Toybox.Application.AppBase {
    var answer as Lang.Long;
    function initialize() {AppBase.initialize(); self.answer = 42l;}
    function onStart(state) {
        var a = new Scalars(); var separate = new Scalars();
        a.classes(); a.absent(); a.change(); a.show(); separate.show();
        a.snapshots(); a.absent(); System.println(self.answer);
        System.exit();
    }
    function getInitialView() {return [new Toybox.WatchUi.View()];}
}
