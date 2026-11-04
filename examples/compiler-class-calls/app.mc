using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
using Nested as Alias;
module Trace {
    var LOG as Lang.Number = 0;
    function mark(n as Lang.Number) as Lang.Number { LOG = LOG * 10 + n; return n; }
}
class Cold {
    static var VALUE as Lang.Number = Trace.mark(2);
    static const OWNER = self;
    static function add(n as Lang.Number) as Lang.Number { VALUE += n; return VALUE; }
    static function owner() { return self; }
    static function clear() as Void { VALUE = 0; }
    static function selfAsObject() as Lang.Object { return self as Lang.Object; }
    static function selfAsCold() as Cold { return self as Cold; }
}
class Token {
    var value as Lang.Number;
    function initialize(n as Lang.Number) { value = n; }
}
class Factory {
    static function make(n as Lang.Number) as Token { return new Token(n); }
    static function value(t as Token) as Lang.Number { return t.value; }
}
module Nested {
    class Methods {
        static function twice(n as Lang.Number) as Lang.Number { return n * 2; }
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        System.println(Trace.LOG);
        System.println(Cold.add(Trace.mark(3)));
        System.println(Trace.LOG);
        Cold.clear();
        System.println(Cold.add(true ? Trace.mark(4) : Trace.mark(5)));
        System.println(Trace.LOG);
        System.println(Cold.owner() == Cold.OWNER);
        System.println(Cold.owner() == Cold);
        var cold = new Cold();
        System.println(cold.owner() == cold);
        System.println(Cold.selfAsObject() instanceof Lang.Object);
        System.println(cold.selfAsObject() instanceof Lang.Object);
        System.println(Cold.selfAsCold() instanceof Cold);
        System.println(cold.selfAsCold() instanceof Cold);
        System.println(Alias.Methods.twice(4));
        System.println($.Nested.Methods.twice(true ? 5 : 6));
        System.println(Factory.value(new Token(7)));
        var token = Factory.make(true ? 8 : 9);
        System.println(token.value);
        Cold.clear();
        System.println(Cold.add(1));
        System.exit();
    }
}
