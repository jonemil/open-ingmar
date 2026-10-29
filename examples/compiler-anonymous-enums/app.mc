using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
const EARLY = AFTER_BASE, GROUPED_EARLY = GROUPED, POSITIVE_EARLY = POSITIVE;
enum {
    ZERO, ONE, BASE = 10, AFTER_BASE,
    CALCULATED = 2 + 3, AFTER_CALCULATED,
    GROUPED = (20), AFTER_GROUPED,
    POSITIVE = +30, AFTER_POSITIVE,
    NEGATIVE = -2, AFTER_NEGATIVE,
    MAXIMUM = 2147483647, WRAPPED,
}
enum { RESET, RESET_NEXT }
const ORDINARY_EARLY = ORDINARY_GROUPED, ORDINARY_GROUPED = (30);
module Trace {
    function number(n as Lang.Number, value as Lang.Number) as Lang.Number {
        System.println(n);
        return value;
    }
}
module M {
    const EARLY = AFTER_BASE, GROUPED_EARLY = GROUPED, POSITIVE_EARLY = POSITIVE;
    enum {
        ZERO, ONE, BASE = 10, AFTER_BASE,
        CALCULATED = 2 + 3, AFTER_CALCULATED,
        GROUPED = (20), AFTER_GROUPED,
        POSITIVE = +30, AFTER_POSITIVE,
        NEGATIVE = -2, AFTER_NEGATIVE,
        MAXIMUM = 2147483647, WRAPPED,
    }
    enum { RESET, RESET_NEXT }
    enum { RUNTIME = Trace.number(90, 4), AFTER_RUNTIME,
           SECOND = Trace.number(91, 6), AFTER_SECOND }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(EARLY);
        System.println(GROUPED_EARLY == null);
        System.println(POSITIVE_EARLY);
        System.println(ZERO);
        System.println(ONE);
        System.println(AFTER_BASE);
        System.println(CALCULATED);
        System.println(AFTER_CALCULATED);
        System.println(GROUPED);
        System.println(AFTER_GROUPED);
        System.println(POSITIVE);
        System.println(AFTER_POSITIVE);
        System.println(NEGATIVE);
        System.println(AFTER_NEGATIVE);
        System.println(MAXIMUM);
        System.println(WRAPPED);
        System.println(RESET);
        System.println(RESET_NEXT);
        System.println(M.EARLY);
        System.println(M.GROUPED_EARLY == null);
        System.println(M.POSITIVE_EARLY);
        System.println(M.ZERO);
        System.println(M.ONE);
        System.println(M.AFTER_BASE);
        System.println(M.CALCULATED);
        System.println(M.AFTER_CALCULATED);
        System.println(M.GROUPED);
        System.println(M.AFTER_GROUPED);
        System.println(M.POSITIVE);
        System.println(M.AFTER_POSITIVE);
        System.println(M.NEGATIVE);
        System.println(M.AFTER_NEGATIVE);
        System.println(M.MAXIMUM);
        System.println(M.WRAPPED);
        System.println(M.RESET);
        System.println(M.RESET_NEXT);
        System.println(M.RUNTIME);
        System.println(M.AFTER_RUNTIME);
        System.println(M.SECOND);
        System.println(M.AFTER_SECOND);
        System.println(M.EXTRA);
        System.println(M.AFTER_EXTRA);
        System.println(ORDINARY_EARLY == null);
        System.println(ORDINARY_GROUPED);
        System.println(M.AFTER_BASE.toString());
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
