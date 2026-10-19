using Toybox.Lang;
using Toybox.System;

class NumericBox {
    var number as Lang.Numeric;
    var integer as Lang.Integer;
    var decimal as Lang.Decimal;
    var maybeNumber as Lang.Numeric or Null;
    var maybeInteger as Lang.Integer or Null = null;
    var maybeDecimal as Lang.Decimal or Null;

    function initialize(n as Lang.Numeric, i as Lang.Integer, d as Lang.Decimal) {
        self.number = n;
        self.integer = i;
        self.decimal = d;
    }
    function write(n as Lang.Numeric, i as Lang.Integer, d as Lang.Decimal) as Void {
        self.number = n;
        self.integer = i;
        self.decimal = d;
    }
    function bump() as Void {
        self.number += 1;
        self.integer++;
        ++self.decimal;
    }
    function numberValue() as Lang.Numeric { return self.number; }
    function integerValue() as Lang.Integer { return self.integer; }
    function decimalValue() as Lang.Decimal { return self.decimal; }
    function nullableNumber() as Lang.Numeric or Null { return self.maybeNumber; }
    function nullableInteger() as Lang.Integer or Null { return self.maybeInteger; }
    function nullableDecimal() as Lang.Decimal or Null { return self.maybeDecimal; }
    function fill() as Void {
        self.maybeNumber = self.number;
        self.maybeInteger = self.integer;
        self.maybeDecimal = self.decimal;
    }
    function clear() as Void {
        self.maybeNumber = null;
        self.maybeInteger = null;
        self.maybeDecimal = null;
    }
    function show() as Void {
        System.println(Fields.describe(self.number));
        System.println(Fields.describe(self.integer));
        System.println(Fields.describe(self.decimal));
    }
    function snapshot() as Void {
        var n = self.maybeNumber;
        var i = self.maybeInteger;
        var d = self.maybeDecimal;
        self.clear();
        if (n != null) { System.println(Fields.describe(n + 1)); }
        if (i != null) { System.println(Fields.describe(i & 7)); }
        if (d != null) { System.println(Fields.describe(d * 2)); }
    }
}
module Fields {
    function describe(n as Lang.Numeric) as Lang.String {
        if (n instanceof Lang.Number) { return "N:" + n; }
        if (n instanceof Lang.Long) { return "L:" + n; }
        if (n instanceof Lang.Float) { return "F:" + n; }
        return "D:" + n;
    }
    function run() as Void {
        var box = new NumericBox(1, 2l, 1.5);
        box.show();
        System.println(box.nullableNumber() == null);
        System.println(box.nullableInteger() == null);
        System.println(box.nullableDecimal() == null);
        box.write(3l, 4, 2.5d);
        box.bump();
        box.show();
        box.fill();
        box.snapshot();
        System.println(box.nullableNumber() == null);
        System.println(box.nullableInteger() == null);
        System.println(box.nullableDecimal() == null);
        var other = new NumericBox(9.5d, 10l, 11.5);
        other.show();
        box.show();
        var value = box.numberValue();
        box.write(100.5, 101l, 102.5d);
        System.println(Fields.describe(value));
        box.show();
    }
}
class MinimalConsoleApp extends Toybox.Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Fields.run(); System.exit(); }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
