using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
class Token {
    var number as Lang.Number;
    function initialize(n as Lang.Number) { self.number = n; }
    function value() as Lang.Number { return self.number; }
    function set(n as Lang.Number) as Void { self.number = n; }
}
class Other { function value() as Lang.Number { return 99; } }
class Order {
    function compare(a as Lang.Object, b as Lang.Object) as Lang.Number {
        var x = a as Token;
        var y = b as Token;
        return x.value() - y.value();
    }
}
typedef Tokens as Lang.Array<Token>;
typedef MaybeTokens as Lang.Array<Token or Null>;
module SourceArrays {
    function tokens() as Tokens { return [new Token(3), new Token(1), new Token(2)]; }
    function optional() as MaybeTokens { return [new Token(7), null]; }
    function nulls() as MaybeTokens { return [null]; }
    function allocated() as MaybeTokens { return new[2]; }
    function nested() as Lang.Array<Tokens> { return [[new Token(9)]]; }
    function union() as Lang.Array<Token or Other> { return [new Token(8), new Other()]; }
    function append(x as [Token], tail as Tokens) as Tokens { x.addAll(tail); return x; }
    function write(x as [Token], i as Lang.Number) as Tokens { x[i] = new Token(10); return x; }
    function loop(x as [Token], n as Lang.Number) as Tokens {
        for (var i = 0; i < n; i++) { x.add(new Token(i)); }
        return x;
    }
    function mixed(x as [Token, Lang.String], i as Lang.Number) as Lang.Array<Token or Lang.String> {
        x[i] = new Token(11);
        return x;
    }
    function run() as Void {
        var xs = SourceArrays.tokens();
        var first = xs[0];
        var reversed = xs.reverse();
        reversed[2].set(30);
        System.println(xs[0].value());
        reversed[2] = new Token(40);
        System.println(xs[0].value());
        var slice = xs.slice(0, 1);
        slice[0].set(31);
        System.println(xs[0].value());
        xs.sort(new Order());
        System.println(xs[0].value());
        System.println(xs[2].value());
        xs.add(first);
        System.println(xs.indexOf(first));
        System.println(xs.remove(first));
        System.println(xs.size());
        System.println(xs.removeAll(first));
        System.println(xs.size());
        System.println(xs[0].value());
        xs.addAll([new Token(4)]);
        System.println(xs[2].value());
        xs[0] = new Token(5);
        System.println(xs[0].value());
        var optional = SourceArrays.optional();
        optional.add(null);
        System.println(optional.remove(null));
        System.println(optional.removeAll(null));
        var present = optional[0];
        if (present != null) { System.println(present.value()); }
        System.println(SourceArrays.allocated()[1] == null);
        var nulls = SourceArrays.nulls();
        nulls[0] = new Token(12);
        System.println(SourceArrays.nulls()[0] == null);
        System.println(SourceArrays.union()[1].value());
        System.println(SourceArrays.nested()[0][0].value());
        System.println(SourceArrays.append([new Token(1)], [new Token(2)]).size());
        System.println(SourceArrays.write([new Token(1)], 0)[0].value());
        System.println(SourceArrays.loop([new Token(1)], 2).size());
        var mixed = SourceArrays.mixed([new Token(1), "text"], 0);
        var value = mixed[0];
        if (value instanceof Token) { System.println(value.value()); }
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { SourceArrays.run(); System.exit(); }
}
