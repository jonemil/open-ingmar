using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
function globalNumber(n as Lang.Number) as Lang.Number { return n + 1; }
module Trace {
    var LOG as Lang.Number = 0;
    function mark(n as Lang.Number) as Lang.Number { LOG = LOG * 1000 + n; return n; }
}
module Cold {
    var MARK as Lang.Number = Trace.mark(101);
    module Child {
        var MARK as Lang.Number = Trace.mark(102);
        function number(n as Lang.Number) as Lang.Number { Trace.mark(103); return n + 10; }
        function owner() { return self; }
    }
}
class Token {
    var value as Lang.Number;
    function initialize(n as Lang.Number) { value = n; System.println(n); }
    function plain() as Lang.Number { return value; }
}
module Calls {
    function receiver(token as Token) as Token { System.println("receiver"); return token; }
    function selector(symbol as Lang.Symbol) as Lang.Symbol { System.println("selector"); return symbol; }
    function isModule(value) as Lang.Boolean { return value == Cold.Child; }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        System.println(Trace.LOG);
        var callback = new Lang.Method(Cold.Child, :number);
        System.println(Trace.LOG);
        System.println(callback.invoke(7) as Lang.Number);
        System.println(Trace.LOG);
        var globalCallback = new Lang.Method($, :globalNumber);
        System.println(globalCallback.invoke(3) as Lang.Number);
        var token = new Token(5);
        var instanceCallback = new Lang.Method(token, :plain);
        token = new Token(9);
        System.println(instanceCallback.invoke() as Lang.Number);
        var dynamicCallback = new Lang.Method(Calls.receiver(token), Calls.selector(true ? :plain : :plain));
        System.println(dynamicCallback.invoke() as Lang.Number);
        var conditional = new Lang.Method(true ? new Token(11) : new Token(12), true ? :plain : :plain);
        System.println(conditional.invoke() as Lang.Number);
        System.println(new Lang.Method(-7, :abs).invoke() as Lang.Number);
        System.println(new Lang.Method("abc", :length).invoke() as Lang.Number);
        var items = [1, 2];
        var weak = items.weak();
        var size = new Lang.Method(items, :size);
        var retained = size;
        items = [0];
        System.println(weak.stillAlive());
        System.println(size.invoke() as Lang.Number);
        size = new Lang.Method(items, :size);
        System.println(weak.stillAlive());
        retained = new Lang.Method(items, :size);
        System.println(weak.stillAlive());
        var classBinding = new Lang.Method(Token, :plain);
        System.println(classBinding instanceof Lang.Method);
        var moduleOwner = new Lang.Method(Cold.Child, :owner);
        System.println(Calls.isModule(moduleOwner.invoke()));
        System.exit();
    }
}
