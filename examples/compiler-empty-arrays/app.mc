using Toybox.System;
module Empty {
    function numbers() as Toybox.Lang.Array<Toybox.Lang.Number> {
        return [];
    }
    function chars() as Toybox.Lang.Array<Toybox.Lang.Char> {
        return ([]);
    }
    function count(a as Toybox.Lang.Array<Toybox.Lang.Number>) {
        return a.size();
    }
    function show(a as Toybox.Lang.Array<Toybox.Lang.Char>) {
        System.println(a.size());
    }
    function effect(n) {
        System.println(n);
        return n;
    }
    function ordered(n, a as Toybox.Lang.Array<Toybox.Lang.Number>, m) {
        return n + a.size() + m;
    }
}
function start() {
    var a = Empty.numbers();
    System.println(a.size());
    a.add(7);
    System.println(a.toString());
    var shared = a;
    a = [];
    System.println(a.size());
    System.println(shared.toString());
    a.addAll([]);
    System.println(a.size());
    System.println(Empty.count([]));
    Empty.show([]);
    var chars = Empty.chars();
    chars.add('x');
    chars.addAll([]);
    System.println(chars.toString());
    chars = [];
    System.println(chars.size());
    System.println(Empty.ordered(Empty.effect(10), [], Empty.effect(20)));
    System.exit();
}
