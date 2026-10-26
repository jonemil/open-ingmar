using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
class Empty {}
module Checks {
    function nativeLength(x as Lang.Number) as Lang.Number { return x.length(); }
    function sourceRead(x as Empty) as Lang.Number { return x.read(); }
    function fieldRead(x as Empty) as Lang.Number { return x.value; }
    function fieldWrite(x as Empty, value as Lang.Number) as Void { x.value = value; }
    function fieldStep(x as Empty) as Lang.Number { x.value += 1; return x.value; }
    function fieldChoice(x as Empty, choose as Lang.Boolean) as Void {
        x.value = choose ? 43 : 44;
    }
    function indexedRead(x as Lang.Number) as Lang.Number { return x[0]; }
    function indexedWrite(x as Lang.Number) as Void { x[0] = 42; }
    function result() as Lang.Number { return "value"; }
    function subtract(x as Lang.String) as Lang.Number { return x - 1; }
    function negate(x as Lang.String) as Lang.Number { return -x; }
    function complement(x as Lang.String) as Lang.Number { return ~x; }
    function index(a as Lang.Array<Lang.Number>, i as Lang.String) as Lang.Number {
        return a[i];
    }
    function condition(x as Lang.Number) as Lang.Number {
        if (x) { return 41; }
        return 0;
    }
    function storage() as Lang.Number {
        var x = 0;
        x = "local";
        return x;
    }
    function replace(x as Lang.Number) as Lang.Number { x = "parameter"; return x; }
}
class Cell {
    var value as Lang.Number;
    function initialize() { self.value = "field"; }
    function read() as Lang.Number { return self.value; }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(Checks.result());
        System.println(Checks.subtract(42));
        System.println(Checks.negate(3));
        System.println(Checks.complement(0));
        System.println(Checks.index([41], 0));
        System.println(Checks.condition(true));
        System.println(Checks.storage());
        System.println(Checks.replace(0));
        System.println(new Cell().read());
        System.println(Checks.nativeLength("three"));
        var cell = new Cell();
        System.println(Checks.sourceRead(cell));
        System.println(Checks.fieldRead(cell));
        Checks.fieldWrite(cell, 41);
        System.println(Checks.fieldStep(cell));
        Checks.fieldChoice(cell, true);
        System.println(Checks.fieldRead(cell));
        var values = [41];
        System.println(Checks.indexedRead(values));
        Checks.indexedWrite(values);
        System.println(Checks.indexedRead(values));
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
