using Toybox.Application;
using Toybox.System;
module Choices {
    function test(value as Toybox.Lang.Boolean) as Toybox.Lang.Boolean { System.println("test"); return value; }
    function yes() as Toybox.Lang.Number { System.println("yes"); return 11; }
    function no() as Toybox.Lang.Number { System.println("no"); return 22; }
    function first() as Toybox.Lang.Number { System.println("first"); return 10; }
    function last() as Toybox.Lang.Number { System.println("last"); return 30; }
    function sum(a as Toybox.Lang.Number, b as Toybox.Lang.Number, c as Toybox.Lang.Number) as Toybox.Lang.Number { return a + b + c; }
    function chars(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Toybox.Lang.Array<Toybox.Lang.Char> { return a; }
    function empty(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { return a.size(); }
    function nullable(b as Toybox.Lang.Boolean) as Toybox.Lang.Number or Null { return b ? 7 : null; }
    function orZero(n as Toybox.Lang.Number or Null) as Toybox.Lang.Number { return n != null ? n : 0; }
    function orText(s as Toybox.Lang.String or Null) as Toybox.Lang.String { return s == null ? "fallback" : s; }
    function nested(n as Toybox.Lang.Number or Null, b as Toybox.Lang.Boolean) as Toybox.Lang.Number { return n != null ? (b ? n : n) : 0; }
    function loop(b as Toybox.Lang.Boolean) as Toybox.Lang.Number {
        var n = "7".toNumber();
        var result = 0;
        while (n != null) { result = b ? n : 0; n = null; }
        return result;
    }
}
class MinimalConsoleApp extends Application.AppBase {
    var count = 0;
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(Choices.test(true) ? Choices.yes() : Choices.no());
        System.println(Choices.test(false) ? Choices.yes() : Choices.no());
        System.println(true ? false : true ? true : true);
        System.println(true ? false ? 1 : 2 : 3);
        System.println(false or true ? 1 : 2);
        System.println(true ? 1 : 2 + 3);
        System.println(1 + (false ? 2 : 3));
        System.println(Choices.sum(Choices.first(), true ? Choices.yes() : Choices.no(), Choices.last()));
        System.println(true ? 'a' : 'b');
        System.println((false ? 'a' : 'b').toNumber());
        var text = true ? "yes" : null;
        if (text != null) { System.println(text); }
        text = false ? "yes" : null;
        if (text != null) { System.println(text); } else { System.println("null"); }
        var n = Choices.nullable(true);
        System.println(n != null ? n + 1 : 0);
        var chars = ['c'];
        var shared = true ? Choices.chars(chars) : ['x'];
        shared[0]++; System.println(chars[0]);
        System.println(Choices.empty(false ? [1] : []));
        self.count = true ? 13 : 14;
        System.println(self.count);
        var numbers = [true ? 1 : 2, false ? 3 : 4];
        System.println(numbers[0]); System.println(numbers[1]);
        var i = 0;
        while (i < 2) { System.println(i == 0 ? "zero" : "one"); i++; }
        System.println(Choices.orZero(7)); System.println(Choices.orZero(null));
        System.println(Choices.orText("value")); System.println(Choices.orText(null));
        System.println(Choices.nested(7, true)); System.println(Choices.loop(true));
        var guarded = [n != null ? n : 0]; System.println(guarded[0]);
        System.println(false and Choices.test(true));
        System.println(true or Choices.test(false));
        System.println(false || true and false);
        System.println(true and Choices.test(false) or Choices.test(true));
        System.println(!0); System.println(!1); System.println(!(-1));
        System.println(!2147483647); System.println(!(-2147483648));
        System.println(!!7); System.println((!7) == (~7));
        if (n != null and n > 0) { System.println(!n); }
        System.exit();
    }
}
