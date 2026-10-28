using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
const BASE = Later.VALUE;
const ANSWER = BASE + 1;
const REAL = 1.5;
const READY = !false;
const CHARACTER = 'A';
const KEY = :value;
const EMPTY = null;
const WRAP = 0x7fffffff + 1;
const SHIFT = -1 >> 31;
const BITS = (6 & 3) | 8;
module Values {
    const ANSWER = 7;
    const NEXT = ANSWER + 1;
    function number() as Lang.Number { return NEXT; }
}
class Counter {
    const OFFSET = 3;
    function number() as Lang.Number { return OFFSET + self.OFFSET; }
}
module Factory {
    function counter() as Counter { System.println(44); return new Counter(); }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(ANSWER);
        System.println(Values.NEXT);
        System.println(Values.number());
        System.println(new Counter().number());
        System.println(Factory.counter().OFFSET);
        var ANSWER = 99;
        ANSWER += 1;
        System.println(ANSWER);
        System.println(REAL == 1.5);
        System.println(READY);
        System.println(CHARACTER.toNumber());
        var values = {KEY=>41};
        var value = values[:value];
        if (value != null) { System.println(value); }
        System.println(EMPTY == null);
        System.println(WRAP);
        System.println(WRAP instanceof Lang.Number);
        System.println(SHIFT);
        System.println(BITS);
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
