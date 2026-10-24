using Toybox.Application;
using Toybox.Lang;
using Toybox.System;

typedef Value as interface {
    var number as Lang.Number;
    function value() as Lang.Number;
};
typedef Pair as [Value, Lang.String];
typedef Visit as Method(x as [Value]) as Lang.Number;

class Token {
    var number as Lang.Number;
    function initialize(n as Lang.Number) { self.number = n; }
    function value() as Lang.Number { return self.number; }
    function compareTo(other as Lang.Object) as Lang.Number {
        var token = other as Token;
        return self.number - token.value();
    }
}
class Order {
    function compare(a as Lang.Object, b as Lang.Object) as Lang.Number { return 0; }
}
class Holder {
    var pair as Pair;
    function initialize() { self.pair = [new Token(45), "field"]; }
    function read() as Lang.Number { return self.pair[0].value(); }
}
class Worker {
    function take(x as [Value]) as Lang.Number { return x[0].value(); }
}
module TupleInterfaces {
    function make() as Pair { return [new Token(41), "direct"]; }
    function inferred() as [Value] { var xs = [new Token(42)]; return xs; }
    function take(x as [Value]) as Lang.Number { return x[0].value(); }
    function maybe(x as Token or Null) as [Value or Null] { return [x]; }
    function nested() as [[Value]] { return [[new Token(43)]]; }
    function rows() as Lang.Array<[Value]> { return [[new Token(44)]]; }
    function callback(visit as Visit) as Lang.Number { return visit.invoke([new Token(46)]); }
    function comparable() as [Lang.Comparable] { return [new Token(47)]; }
    function comparator() as [Lang.Comparator] { return [new Order()]; }
    function text() as [interface {function toString() as Lang.String;}] { return [48]; }
    function run() as Void {
        var pair = make();
        System.println(pair[0].value());
        System.println(pair[1]);
        pair[0].number = 51;
        System.println(pair[0].value());
        System.println(inferred()[0].value());
        System.println(take([new Token(50)]));
        System.println(nested()[0][0].value());
        System.println(rows()[0][0].value());
        System.println(new Holder().read());
        var worker = new Worker();
        System.println(callback(worker.method(:take)));
        System.println(worker.take([new Token(52)]));
        var optional = maybe(new Token(53))[0];
        if (optional != null) { System.println(optional.value()); }
        System.println(maybe(null)[0] == null);
        System.println(comparable()[0].compareTo(new Token(7)));
        System.println(comparator()[0].compare(1, 2));
        System.println(text()[0].toString());
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { TupleInterfaces.run(); System.exit(); }
}
