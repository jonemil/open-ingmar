using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
const EARLY = CHOICE;
const CHOICE = true ? Trace.text(1, "chosen") : Trace.text(99, "unused");
const SKIP_AND = false && Trace.flag(98, true);
const SKIP_OR = true || Trace.flag(97, false);
const TAKE_AND = true and Trace.flag(2, false);
const TAKE_OR = false or Trace.flag(3, true);
const ZERO_AND = false && (1 / 0 == 0);
const ZERO_CHOICE = true ? 7 : 1 / 0;
const ITEMS = Trace.flag(4, false) ? Trace.array(96, 1) : Trace.array(5, 8);
const ALIAS = ITEMS;
const FORMAT = true ? Lang.NUMBER_FORMAT_UINT16 : Lang.NUMBER_FORMAT_SINT16;
const NESTED = Trace.flag(6, true) ? (Trace.flag(7, false) ? 99 : 9) : 98;
const LATER = Trace.array(8, 10);
module Trace {
    function flag(n as Lang.Number, value as Lang.Boolean) as Lang.Boolean {
        System.println(n);
        return value;
    }
    function text(n as Lang.Number, value as Lang.String) as Lang.String {
        System.println(n);
        return value;
    }
    function array(n as Lang.Number, value as Lang.Number) as Lang.Array<Lang.Number> {
        System.println(n);
        return [value];
    }
}
module M {
    const EARLY = CHOICE;
    const CHOICE = true ? Trace.text(1, "chosen") : Trace.text(99, "unused");
    const SKIP_AND = false && Trace.flag(98, true);
    const SKIP_OR = true || Trace.flag(97, false);
    const TAKE_AND = true and Trace.flag(2, false);
    const TAKE_OR = false or Trace.flag(3, true);
    const ZERO_AND = false && (1 / 0 == 0);
    const ZERO_CHOICE = true ? 7 : 1 / 0;
    const ITEMS = Trace.flag(4, false) ? Trace.array(96, 1) : Trace.array(5, 8);
    const ALIAS = ITEMS;
    const FORMAT = true ? Lang.NUMBER_FORMAT_UINT16 : Lang.NUMBER_FORMAT_SINT16;
    const NESTED = Trace.flag(6, true) ? (Trace.flag(7, false) ? 99 : 9) : 98;
    const LATER = Trace.array(8, 10);
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(EARLY == null);
        System.println(CHOICE);
        System.println(SKIP_AND);
        System.println(SKIP_OR);
        System.println(TAKE_AND);
        System.println(TAKE_OR);
        System.println(ZERO_AND);
        System.println(ZERO_CHOICE);
        System.println(ITEMS[0]);
        System.println(NESTED);
        System.println(LATER[0]);
        System.println(M.EARLY == null);
        System.println(M.CHOICE);
        System.println(M.SKIP_AND);
        System.println(M.SKIP_OR);
        System.println(M.TAKE_AND);
        System.println(M.TAKE_OR);
        System.println(M.ZERO_AND);
        System.println(M.ZERO_CHOICE);
        System.println(M.ITEMS[0]);
        System.println(M.NESTED);
        System.println(M.LATER[0]);
        M.ALIAS[0] = 11;
        System.println(M.ITEMS[0]);
        System.println(ITEMS[0]);
        var bytes = [0, 0]b;
        bytes.encodeNumber(513, M.FORMAT, {});
        System.println(bytes.decodeNumber(FORMAT, {}).toString());
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
