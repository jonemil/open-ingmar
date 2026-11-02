using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
var VALUE as Lang.Number = 3;
const LITERAL = 11;
typedef GlobalToken as Token;
var ITEMS as Lang.Array<Lang.Number> = [1, 2];
var TOKEN as Token = new Token(19);
function rootCall(n as Lang.Number) as Lang.Number { return n + 1; }
module Trace {
    function mark(n as Lang.Number) as Lang.Number { System.println(n); return n; }
}
module Names {
    var MARK as Lang.Number = Trace.mark(101);
    var VALUE as Lang.Number = 5;
    module Child {
        var MARK as Lang.Number = Trace.mark(102);
        var VALUE as Lang.Number = 7;
        var TOKEN as Token = new Token(23);
        var ITEMS as Lang.Array<Lang.Number> = [3, 4];
        function number(n as Lang.Number) as Lang.Number { return n; }
    }
}
class Token {
    var value as Lang.Number;
    enum Kind { SMALL = 17 }
    function initialize(n as Lang.Number) { value = n; }
    function get() as Lang.Number { return value; }
}
module Shadow {
    class Token { function value() as Lang.Number { return 99; } }
    typedef GlobalToken as Lang.Number;
    function typed(token as $.GlobalToken) as $.Token { return token as $.Token; }
    function create() as $.Token {
        return new $.Token(37);
    }
    var VALUE as Lang.Number = 99;
    const LITERAL = 100;
    module Names { var VALUE as Lang.Number = 88; }
    function read(Names as Lang.Number) as Lang.Number {
        var VALUE = 50;
        return $.VALUE + $.LITERAL + $.Names.Child.VALUE + $.rootCall(Names);
    }
    function write(flag as Lang.Boolean) as Void {
        $.VALUE += flag ? 2 : 3;
        $.Names.Child.VALUE = Trace.mark(103) > 0 ? 9 : 10;
        $.ITEMS[Trace.mark(104) - 104] += Trace.mark(105) > 0 ? 4 : 5;
        $.Names.Child.TOKEN.value += 1;
        $.TOKEN.value = 29;
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        System.println($.Names.Child.VALUE);
        System.println(Shadow.read(5));
        Shadow.write(true);
        System.println($.VALUE);
        System.println($.Names.Child.VALUE);
        System.println($.ITEMS[0]);
        System.println($.ITEMS.size());
        System.println($.TOKEN.get());
        System.println($.Names.Child.TOKEN.get());
        System.println($.Names.Child.ITEMS[1]);
        System.println($.Token.SMALL);
        System.println($.Names.Child.number(Trace.mark(106) > 0 ? 31 : 32));
        $.Toybox.System.println($.Toybox.Lang.NUMBER_FORMAT_UINT8 as Lang.Number);
        var rootToken = Shadow.typed(Shadow.create());
        System.println(rootToken.get());
        System.println(rootToken instanceof $.Token);
        System.exit();
    }
}
