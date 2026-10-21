using Toybox.System;
module Writes {
    function receiver(values as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Array<Toybox.Lang.Number> {
        System.println("receiver");
        return values;
    }
    function index() as Toybox.Lang.Number { System.println("index"); return 1; }
    function value(values as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number {
        System.println("value");
        values.remove(10);
        return 99;
    }
    function flag() as Toybox.Lang.Boolean { System.println("flag"); return true; }
    function selected(flag as Toybox.Lang.Boolean) as Toybox.Lang.Number {
        System.println("selected");
        if (flag) { return 7; }
        return 8;
    }
    function fill(values as Toybox.Lang.Array<Toybox.Lang.Number>) as Void {
        var index = 0;
        while (index < values.size()) {
            values[index] = index * 10;
            index = index + 1;
        }
    }
    function run() as Void {
        var values = [10, 20, 30];
        var copied = values;
        System.println("writes");
        Writes.receiver(values)[Writes.index()] = Writes.value(values);
        System.println(copied.toString());
        Writes.receiver(values)[Writes.index()] = Writes.selected(false || Writes.flag());
        System.println(values.toString());
        Writes.fill(values);
        System.println(copied.toString());
        var number = "41".toNumber();
        if (number != null) { values[0] = number; }
        System.println(values[0]);
        System.println(values.size());
        System.println("done");
    }
}
