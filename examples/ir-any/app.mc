using Toybox.Application;
using Toybox.System;
module Dynamic {
    function empty() {}
    function bare() { return; }
    function maybe(b) { if (b) { return 42; } else { return null; } }
    function array(x) { return [x, 7]; }
    function sum(n) {
        var total = 0;
        while (n > 0) { total += n; n -= 1; }
        return total;
    }
    function identity(x) { return x; }
    function length(x) { return x.length(); }
    function add(x, y) { return x + y; }
    function at(x, key) { return x[key]; }
    function put(x, key, value) as Void { x[key] = value; }
    function condition(x) as Toybox.Lang.Boolean {
        if (x) { return true; } else { return false; }
    }
    function custom(x) { return x.value(); }
    function format(x) as Toybox.Lang.String { return 42.format(x); }
    function narrow(x) as Toybox.Lang.Number {
        if (x instanceof Toybox.Lang.Number) { return x + 1; }
        else { return 0; }
    }
}
function main() as Void {
    System.println("dynamic");
    System.println(Dynamic.identity("hi"));
    System.println(Dynamic.identity(null));
    System.println(Dynamic.length("abc"));
    System.println(Dynamic.add(40, 2));
    var a = [42];
    System.println(Dynamic.at(a, 0));
    Dynamic.put(a, 0, 7);
    System.println(Dynamic.at(a, 0));
    System.println(Dynamic.condition(true));
    System.println(Dynamic.custom(new Token()));
    System.println(Dynamic.format("%d"));
    System.println(Dynamic.narrow(41));
    System.println(Dynamic.empty());
    System.println(Dynamic.bare());
    System.println(Dynamic.maybe(true));
    System.println(Dynamic.maybe(false));
    System.println(Dynamic.at(Dynamic.array("x"), 0));
    System.println(Dynamic.at(Dynamic.array("x"), 1));
    System.println(Dynamic.sum(3));
}
class Token {
    function initialize() {}
    function value() as Toybox.Lang.Number { return 42; }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { main(); System.exit(); }
}
