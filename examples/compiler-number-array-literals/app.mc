using Toybox.System;
module Literals {
    function effect(n) {
        System.println(n);
        return n;
    }
    function make(n) as Toybox.Lang.Array<Toybox.Lang.Number> {
        return [n, -n, n + 1];
    }
    function first(a as Toybox.Lang.Array<Toybox.Lang.Number>) {
        return a[0];
    }
}
function start() {
    var n = 7;
    System.println([n, 8, 9].toString());
    System.println(Literals.make(3).toString());
    System.println([Literals.effect(10), Literals.effect(20)].toString());
    System.println([(-5).abs(), 'A'.toNumber()].toString());
    System.println(Literals.first([40 + 2, 0]));
    var found = "abc".find("b");
    if (!(found == null)) {
        System.println([found, found + 1].toString());
    }
    var text = "12".substring(0, 2);
    if (!(text == null)) {
        var number = text.toNumber();
        if (!(number == null)) {
            System.println([number, number + 1].toString());
        }
    }
    System.println(['a' + 1, 'z'].toString());
    System.exit();
}
