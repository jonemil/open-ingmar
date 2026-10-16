using Toybox.Application;
using Toybox.System;

module Wide {
    function pass(value as Toybox.Lang.Long) as Toybox.Lang.Long { return value; }
    function zero() as Toybox.Lang.Long { return 0l; }
    function minimum() as Toybox.Lang.Long { return -9223372036854775808l; }
    function maximum() as Toybox.Lang.Long { return 9223372036854775807L; }
}

class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function pass(value as Toybox.Lang.Long) as Toybox.Lang.Long { return value; }
    function onStart(state) {
        System.println(0l);
        System.println(1l);
        System.println(-1l);
        System.println(2147483648l);
        System.println(4294967296l);
        System.println(72623859790382856l);
        System.println(Wide.minimum());
        System.println(Wide.maximum());
        var value = Wide.pass(4294967297l);
        var copied = value;
        value = -4294967297l;
        System.println(copied);
        System.println(self.pass(value));
        System.println(Wide.pass(Wide.zero()));
        Wide.pass(42l);
        var n = 0;
        while (n < 2) { System.println(Wide.pass(256l)); n++; }
        System.exit();
    }
}
