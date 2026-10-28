using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
const ROOT_LATER = new Later().VALUE;
const ROOT_ARRAY = LATE_ARRAY;
const ROOT_TEXT = LATE_TEXT;
const ROOT_LONG = LATE_LONG;
const LATE_ARRAY = [7];
const LATE_TEXT = "root";
const LATE_LONG = 2147483648l;
module M {
    const ARRAY = LATE_ARRAY;
    const BYTES = LATE_BYTES;
    const WIDE = LATE_WIDE;
    const MID_TEXT = LATE_TEXT;
    const TEXT = MID_TEXT;
    const LATE_ARRAY = [8];
    const LATE_BYTES = [1,2]b;
    const LATE_WIDE = [2147483648l];
    const LATE_TEXT = "module";
}
module Trace {
    function make(n as Lang.Number) as Lang.Array<Lang.Number> {
        System.println(n);
        return [n];
    }
}
class First {
    const VALUE = new Later().VALUE;
    const FIRST = Trace.make(1);
    const MID_TEXT = LATE_TEXT;
    const TEXT = MID_TEXT;
    const LONG = LATE_LONG;
    const LATE_TEXT = "class";
    const LATE_LONG = 2147483648l;
    const SECOND = Trace.make(2);
    function initialize() { System.println(3); }
}
class Later {
    const VALUE = [41];
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(ROOT_LATER[0]);
        ROOT_ARRAY[0] = 9;
        System.println(LATE_ARRAY[0]);
        System.println(ROOT_TEXT == LATE_TEXT);
        System.println(ROOT_LONG == 2147483648l);
        System.println(M.ARRAY[0]);
        M.BYTES[0] = 9;
        System.println(M.LATE_BYTES[0]);
        System.println(M.WIDE[0] == 2147483648l);
        System.println(M.TEXT == M.LATE_TEXT);
        var first = new First();
        var second = new First();
        System.println(first.VALUE[0]);
        System.println(first.TEXT == first.LATE_TEXT);
        System.println(first.TEXT == second.TEXT);
        System.println(first.LONG == 2147483648l);
        System.println(first.FIRST[0]);
        System.println(first.SECOND[0]);
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
