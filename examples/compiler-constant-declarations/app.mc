using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
const NUMBER as Count = 1, BARE = 2;
const OBJECT as Lang.Object = 3, NONE as Count or Null, IMPLICIT;
const FORWARD as Lang.String = TEXT, TEXT as Lang.String = "root";
const EMPTY as Lang.Array<Lang.Number> = [];
const TABLE as Lang.Dictionary<Lang.Symbol, Lang.Number> = {:key => 5};
const NESTED as Lang.Array<Lang.Array<Lang.Number>> = [[6]];
const FORMAT as Lang.NumberFormat = Lang.NUMBER_FORMAT_UINT16;
module Trace {
    function number(n as Lang.Number) as Lang.Number {
        System.println(n);
        return n;
    }
}
module M {
    const FIRST as Count = Trace.number(10), SECOND as Count = Trace.number(20);
    const VALUE as Count = 7, COPY = VALUE;
    const VALUES as Lang.Array<Lang.Number> = [8], ALIAS = VALUES;
    const NONE as Count or Null, IMPLICIT;
}
class Box {
    const TEXT as Lang.String = "class", COPY as Lang.String = TEXT;
    const VALUES as Lang.Array<Lang.Number> = [4, 5], ALIAS = VALUES;
    const NONE as Lang.String or Null, IMPLICIT;
}
class Holder {
    const TOKEN as Token = new Token(), MAYBE as MaybeToken;
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(NUMBER);
        System.println(BARE);
        System.println(OBJECT.toString());
        System.println(NONE == null);
        System.println(IMPLICIT == null);
        System.println(FORWARD == TEXT);
        EMPTY.add(9);
        System.println(EMPTY[0]);
        var value = TABLE[:key];
        if (value != null) { System.println(value); }
        System.println(NESTED[0][0]);
        System.println(M.COPY);
        M.ALIAS[0] = 10;
        System.println(M.VALUES[0]);
        System.println(M.NONE == null);
        System.println(M.IMPLICIT == null);
        var first = new Box();
        var second = new Box();
        first.ALIAS[0] = 11;
        System.println(first.VALUES[0]);
        System.println(second.VALUES[0]);
        System.println(first.COPY == first.TEXT);
        System.println(first.NONE == null);
        System.println(first.IMPLICIT == null);
        var holder = new Holder();
        System.println(holder.TOKEN.text());
        System.println(holder.MAYBE == null);
        var bytes = [0, 0]b;
        bytes.encodeNumber(513, FORMAT, {});
        System.println(bytes.decodeNumber(FORMAT, {}).toString());
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
