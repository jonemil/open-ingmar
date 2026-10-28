using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
const EARLY_LITERAL = LITERAL;
const LITERAL = [-1, +2, (3)];
const EARLY_EXPR = EXPR;
const EXPR = [1 + 2];
const EARLY_REF = REF;
const REF = [N];
const N = 7;
const EARLY_BYTES = BYTES;
const BYTES = [N]b;
const EARLY_TEXT = TEXT;
const TEXT = [S];
const S = "scalar";
const EARLY_WIDE = WIDE;
const WIDE = [L];
const L = 2147483648l;
const EARLY_EMPTY = EMPTY;
const EMPTY = [];
const EARLY_EMPTY_BYTES = EMPTY_BYTES;
const EMPTY_BYTES = []b;
const EARLY_SIGNED = SIGNED;
const SIGNED = [-2147483649l, -2.5d, +2147483648l, +2.5d];
const EARLY_GROUPED = GROUPED;
const GROUPED = [-(1)];
const EARLY_SUM = SUM;
const SUM = 4l + 5l;
const EARLY_JOIN = JOIN;
const JOIN = "a" + "b";
const ALIAS = TEXT;
module Trace {
    function number(n as Lang.Number) as Lang.Number {
        System.println(n);
        return n;
    }
}
module M {
    const FIRST = Trace.number(1);
    const EARLY_LITERAL = LITERAL;
    const LITERAL = [-1, +2, (3)];
    const EARLY_EXPR = EXPR;
    const EXPR = [1 + 2];
    const EARLY_REF = REF;
    const REF = [N];
    const N = 7;
    const EARLY_BYTES = BYTES;
    const BYTES = [N]b;
    const EARLY_TEXT = TEXT;
    const TEXT = [S];
    const S = "scalar";
    const EARLY_WIDE = WIDE;
    const WIDE = [L];
    const L = 2147483648l;
    const EARLY_EMPTY = EMPTY;
    const EMPTY = [];
    const EARLY_EMPTY_BYTES = EMPTY_BYTES;
    const EMPTY_BYTES = []b;
    const EARLY_SIGNED = SIGNED;
    const SIGNED = [-2147483649l, -2.5d, +2147483648l, +2.5d];
    const EARLY_GROUPED = GROUPED;
    const GROUPED = [-(1)];
    const EARLY_SUM = SUM;
    const SUM = 4l + 5l;
    const EARLY_JOIN = JOIN;
    const JOIN = "a" + "b";
    const ALIAS = TEXT;
    const CALL = [Trace.number(3)];
    const SECOND = Trace.number(2);
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(EARLY_LITERAL == null);
        System.println(EARLY_EXPR == null);
        System.println(EARLY_REF == null);
        System.println(EARLY_BYTES == null);
        System.println(EARLY_TEXT == null);
        System.println(EARLY_WIDE == null);
        System.println(EARLY_EMPTY == null);
        System.println(EARLY_EMPTY_BYTES == null);
        System.println(EARLY_SIGNED == null);
        System.println(EARLY_GROUPED == null);
        System.println(EARLY_SUM == null);
        System.println(EARLY_JOIN == null);
        System.println(EXPR[0]);
        System.println(REF[0]);
        System.println(BYTES[0]);
        System.println(TEXT[0]);
        System.println(WIDE[0] == 2147483648l);
        System.println(EMPTY.size());
        System.println(EMPTY_BYTES.size());
        System.println(GROUPED[0]);
        System.println(SUM == 9l);
        System.println(JOIN);
        ALIAS[0] = "changed";
        System.println(TEXT[0]);
        System.println(S);
        System.println(M.EARLY_LITERAL == null);
        System.println(M.EARLY_EXPR == null);
        System.println(M.EARLY_REF == null);
        System.println(M.EARLY_BYTES == null);
        System.println(M.EARLY_TEXT == null);
        System.println(M.EARLY_WIDE == null);
        System.println(M.EARLY_EMPTY == null);
        System.println(M.EARLY_EMPTY_BYTES == null);
        System.println(M.EARLY_SIGNED == null);
        System.println(M.EARLY_GROUPED == null);
        System.println(M.EARLY_SUM == null);
        System.println(M.EARLY_JOIN == null);
        System.println(M.EXPR[0]);
        System.println(M.REF[0]);
        System.println(M.BYTES[0]);
        System.println(M.TEXT[0]);
        System.println(M.WIDE[0] == 2147483648l);
        System.println(M.EMPTY.size());
        System.println(M.EMPTY_BYTES.size());
        System.println(M.GROUPED[0]);
        System.println(M.SUM == 9l);
        System.println(M.JOIN);
        M.ALIAS[0] = "changed";
        System.println(M.TEXT[0]);
        System.println(M.S);
        System.println(M.CALL[0]);
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
