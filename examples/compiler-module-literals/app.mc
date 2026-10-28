using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
const STARTUP_VALUE = Startup.VALUE;
module Trace {
    function number(n as Lang.Number) as Lang.Number {
        System.println(n);
        return n;
    }
}
module Startup {
    const EFFECT = Trace.number(90);
    const VALUE = 9;
}
module Numbers {
    const EFFECT = Trace.number(10);
    const COPY = VALUE;
    const EARLY = LATE;
    const LATE = VALUE;
    const VALUE = 7;
    function read() as Lang.Number { return self.VALUE; }
}
module Outer {
    const VALUE = Numbers.VALUE;
    const EFFECT = Trace.number(20);
}
module Floats {
    const EFFECT = Trace.number(30);
    const VALUE = 1.5f;
}
module Booleans {
    const EFFECT = Trace.number(40);
    const VALUE = true;
}
module Chars {
    const EFFECT = Trace.number(50);
    const VALUE = 'A';
}
module Symbols {
    const EFFECT = Trace.number(60);
    const VALUE = :key;
}
module Nulls {
    const EFFECT = Trace.number(70);
    const VALUE = null;
}
class Capture {
    const VALUE = Other.VALUE;
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(STARTUP_VALUE);
        System.println(Startup.VALUE);
        System.println(Outer.VALUE);
        System.println(Numbers.VALUE);
        System.println(Numbers.COPY);
        System.println(Numbers.EARLY == null);
        System.println(Numbers.LATE);
        System.println(Numbers.read());
        System.println(Floats.VALUE);
        System.println(Booleans.VALUE);
        System.println(Chars.VALUE.toNumber());
        System.println(Symbols.VALUE == :key);
        System.println(Nulls.VALUE == null);
        var captured = new Capture();
        System.println(captured.VALUE);
        System.println(Other.VALUE);
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
