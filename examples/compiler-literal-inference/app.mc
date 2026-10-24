using Toybox.Application;
using Toybox.Lang;
using Toybox.System;

class Token {
    var number as Lang.Number;
    function initialize(n as Lang.Number) { self.number = n; }
    function value() as Lang.Number { return self.number; }
}

module Literals {
    function number(n as Lang.Number) as Lang.Number {
        System.println(n);
        return n;
    }
    function numbers() as Lang.Array<Lang.Number> { return [7, 8]; }
    function count(xs as Lang.Array<Lang.Number>) as Lang.Number { return xs.size(); }
    function nestedWrite(xs as [Lang.Array<Lang.Number>], i as Lang.Number)
        as Lang.Array<Lang.Array<Lang.Number>> {
        xs[i] = [35];
        return xs;
    }
    function nestedView(xs as Lang.Array<[Lang.Number]>)
        as Lang.Array<Lang.Array<Lang.Number>> { return xs; }
    function run() as Void {
        var empty = [];
        System.println(empty.size());
        empty.add(1);
        empty.addAll([2, 3]);
        empty[0] += 10;
        System.println(empty[0]);
        System.println(empty[2]);

        var same = [4, 5];
        same[0] = "changed";
        System.println(same[0]);
        System.println(same[1]);
        same = [];
        same.add(:done);
        System.println(same[0] == :done);

        var computed = [number(21), number(22) + 1, "computed"];
        System.println(computed[0]);
        System.println(computed[1]);
        System.println(computed[2]);
        var symbols = [:first, 42, :last];
        System.println(symbols[0] == :first);
        System.println(symbols[1]);
        System.println(symbols.reverse()[0] == :last);

        var objects = [new Token(31), new Token(32)];
        objects.add(new Token(33));
        System.println(objects[2].value());
        var mixed = [new Token(34), :source, null];
        System.println(mixed[0].value());
        System.println(mixed[1] == :source);
        System.println(mixed[2] == null);

        var sorted = [3, 1, 2];
        sorted.sort(null);
        System.println(sorted[0]);
        System.println(sorted.slice(1, 3)[1]);
        System.println(sorted.indexOf(2));
        System.println(sorted.remove(2));
        System.println(sorted.size());

        var loop = [];
        for (var i = 0; i < 3; i++) { loop.add(i); }
        System.println(loop.size());
        System.println(count([1, 2]));
        var typed = numbers();
        typed.addAll([9, 10]);
        System.println(typed[3]);
        System.println([[], []][1].size());
        System.println(nestedWrite([[1, 2]], 0)[0][0]);
        System.println(nestedView([[36]])[0][0]);
        var bytes = []b;
        var tail = [37, 38];
        bytes.addAll(tail);
        bytes.addAll(['A']);
        bytes.addAll([]);
        System.println(bytes[0]);
        System.println(bytes[2]);
    }
}

class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Literals.run(); System.exit(); }
}
