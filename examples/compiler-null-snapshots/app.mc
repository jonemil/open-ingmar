using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
const EARLY = LATE;
const CAPTURED = [LATE];
const PASSED = Trace.pass(LATE);
const WAS_NULL = LATE == null;
const LATE = Trace.make(6);
const DICT_EARLY = DICT;
const DICT = {:value=>7};
const NESTED_EARLY = NESTED;
const NESTED = [[8]];
module Trace {
    function make(n as Lang.Number) as Lang.Array<Lang.Number> {
        System.println(n);
        return [n];
    }
    function text() as Lang.String { System.println(4); return "late"; }
    function pass(value as Lang.Array<Lang.Number> or Null) as Lang.Array<Lang.Number> or Null {
        return value;
    }
}
module M {
    const EARLY = FUTURE;
    const TEXT_EARLY = TEXT;
    const TEXT = Trace.text();
    const DICT_EARLY = DICT;
    const DICT = {:value=>7};
    const NESTED_EARLY = NESTED;
    const NESTED = [[8]];
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(EARLY == null);
        System.println(LATE[0]);
        System.println(DICT_EARLY == null);
        System.println(NESTED_EARLY == null);
        System.println(CAPTURED[0] == null);
        System.println(PASSED == null);
        System.println(WAS_NULL);
        var value = DICT[:value];
        if (value != null) { System.println(value); }
        NESTED[0][0] = 9;
        System.println(NESTED[0][0]);
        System.println(M.EARLY == null);
        System.println(M.FUTURE[0]);
        System.println(M.TEXT_EARLY == null);
        System.println(M.TEXT);
        System.println(M.DICT_EARLY == null);
        System.println(M.NESTED_EARLY == null);
        M.AFTER[0] = 10;
        System.println(M.FUTURE[0]);
        System.println(EARLY == null);
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
