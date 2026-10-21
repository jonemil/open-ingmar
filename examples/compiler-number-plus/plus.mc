using Toybox.System;
module Plus {
    function value(n as Toybox.Lang.Number) as Toybox.Lang.Number { return +n; }
    function effect() as Toybox.Lang.Number { System.println("operand"); return -7; }
    function flag() as Toybox.Lang.Boolean { System.println("flag"); return true; }
    function run() as Void {
        System.println(Plus.value(-2147483648));
        System.println(Plus.value(2147483647));
        System.println(Plus.value(0));
        System.println(+Plus.effect());
        System.println(+1);
        System.println(+(1 + 2) * 3);
        System.println(+(false || Plus.flag()).hashCode());
        System.println(+(~1));
        System.println(+(-1));
        System.println(+(+1));
        System.println(+1.abs());
        System.println(+(+Plus.effect()));
        var n = "41".toNumber();
        if (n != null) { System.println(+n); }
        var values = [0];
        values[0] = +Plus.value(9);
        System.println(values[0]);
        System.println("done");
    }
}
