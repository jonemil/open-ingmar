using Toybox.System;
module Chars {
    function value(c as Toybox.Lang.Char) as Toybox.Lang.Char {
        System.println(c);
        return c;
    }
    function skipped() as Toybox.Lang.Boolean {
        System.println("unexpected");
        return false;
    }
    function make() as Toybox.Lang.Array<Toybox.Lang.Char> {
        return ['p', 'q'];
    }
}
function start() as Void {
    var a = ['a', 'b'];
    var shared = a;
    a.add('c');
    System.println(shared.toString());
    System.println(a[1]);
    var effects = [Chars.value('x'), Chars.value('y')];
    System.println(effects.toString());
    var logical = [(true || Chars.skipped()).hashCode().toChar(), 'z'];
    System.println(logical[0].toNumber());
    System.println(logical[1]);
    System.println(['Ω', 128512.toChar()].size());
    System.println(Chars.make().toString());
    var i = 0;
    while (3 > i) {
        var fresh = ['v'];
        System.println(fresh.size());
        i = i + 1;
    }
    System.exit();
}
