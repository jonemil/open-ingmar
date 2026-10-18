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
    function sized(n) as Toybox.Lang.Array<Toybox.Lang.Number or Null> {
        return new[n];
    }
    function nullableCount(a as Toybox.Lang.Array<Toybox.Lang.Number or Null>) {
        return a.size();
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
    var sized = Empty.sized(Empty.effect(3));
    System.println(sized.toString());
    System.println(sized[0] == null);
    sized[1] = 7;
    var old = sized;
    sized = new[2];
    System.println(sized.toString());
    System.println(old.toString());
    System.println(Empty.nullableCount(new[Empty.effect(0)]));
    sized = new[true ? Empty.effect(1) : Empty.effect(9)];
    System.println(sized.size());
    System.println(sized[0] == null);
    System.exit();
}
