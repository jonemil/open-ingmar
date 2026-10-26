using Toybox.Application;
using Toybox.Lang;
using Toybox.System;

typedef Mixed as Lang.Number or Lang.String;
class WithValue {
    function value() as Lang.Number { return 41; }
    function length() as Lang.Number { return 4; }
    function toString() as Lang.String { return "source"; }
    function touch() as Void { System.println("touched"); }
}
class Missing {}
class TextValue {
    function length() as Lang.String { return "text"; }
}
module Operations {
    function source(x as WithValue or Missing) as Lang.Number { return x.value(); }
    function sourceNullable(x as WithValue or Null) as Lang.Number { return x.value(); }
    function mixed(x as WithValue or Lang.Number) as Lang.Number { return x.value(); }
    function mixedLength(x as WithValue or Lang.String) as Lang.Number { return x.length(); }
    function mixedString(x as WithValue or Lang.String) as Lang.String { return x.toString(); }
    function sourceResults(x as WithValue or TextValue) as Mixed { return x.length(); }
    function effect(x as WithValue or Missing) as Void { x.touch(); }
    function subtract(x as Mixed) as Lang.Number { return x - 1; }
    function negate(x as Mixed) as Lang.Number { return -x; }
    function positive(x as Mixed) as Lang.String { return +x; }
    function complement(x as Mixed) as Lang.Number { return ~x; }
    function increment(x as Mixed) as Lang.Number { ++x; return x; }
    function decrement(x as Mixed) as Lang.Number { --x; return x; }
    function less(x as Mixed) as Lang.Boolean { return x < 1; }
    function read(a as Lang.Array<Lang.Number>, i as Mixed) as Lang.Number {
        return a[i];
    }
    function write(a as Lang.Array<Lang.Number>, i as Mixed) as Void { a[i] = 7; }
    function length(x as Mixed) as Lang.Number { return x.length(); }
    function nullableLength(x as Lang.String or Null) as Lang.Number {
        return x.length();
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(Operations.subtract(42));
        System.println(Operations.negate(2));
        System.println(Operations.positive("plus"));
        System.println(Operations.complement(0));
        System.println(Operations.increment(4));
        System.println(Operations.decrement(4));
        System.println(Operations.less(0));
        var values = [41];
        System.println(Operations.read(values, 0));
        Operations.write(values, 0);
        System.println(values[0]);
        System.println(Operations.length("abc"));
        System.println(Operations.nullableLength("four"));
        var item = new WithValue();
        System.println(Operations.source(item));
        System.println(Operations.sourceNullable(item));
        System.println(Operations.mixed(item));
        System.println(Operations.mixedLength(item));
        System.println(Operations.mixedLength("three"));
        System.println(Operations.mixedString(item));
        System.println(Operations.mixedString("native"));
        System.println(Operations.sourceResults(item));
        System.println(Operations.sourceResults(new TextValue()));
        Operations.effect(item);
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
