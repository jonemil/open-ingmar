using Toybox.Application;
using Toybox.Lang;
using Toybox.System;

typedef Mixed as Lang.Number or Lang.String;
module Operations {
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
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
