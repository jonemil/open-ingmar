using Toybox.System;
module Bits {
    function both(a as Toybox.Lang.Number, b as Toybox.Lang.Number) as Toybox.Lang.Number { return a & b; }
    function either(a as Toybox.Lang.Number, b as Toybox.Lang.Number) as Toybox.Lang.Number { return a | b; }
    function different(a as Toybox.Lang.Number, b as Toybox.Lang.Number) as Toybox.Lang.Number { return a ^ b; }
    function left(a as Toybox.Lang.Number, b as Toybox.Lang.Number) as Toybox.Lang.Number { return a << b; }
    function right(a as Toybox.Lang.Number, b as Toybox.Lang.Number) as Toybox.Lang.Number { return a >> b; }
    function complement(a as Toybox.Lang.Number) as Toybox.Lang.Number { return ~a; }
    function order(a as Toybox.Lang.Number, b as Toybox.Lang.Number, c as Toybox.Lang.Number) as Toybox.Lang.Number { return a + b << c; }
    function product(a as Toybox.Lang.Number, b as Toybox.Lang.Number, c as Toybox.Lang.Number) as Toybox.Lang.Number { return a << b * c; }
    function masks(a as Toybox.Lang.Number, b as Toybox.Lang.Number, c as Toybox.Lang.Number) as Toybox.Lang.Number { return a | b & c; }
    function xors(a as Toybox.Lang.Number, b as Toybox.Lang.Number, c as Toybox.Lang.Number) as Toybox.Lang.Number { return a | b ^ c; }
    function addOr(a as Toybox.Lang.Number, b as Toybox.Lang.Number, c as Toybox.Lang.Number) as Toybox.Lang.Number { return a + b | c; }
    function xorAdd(a as Toybox.Lang.Number, b as Toybox.Lang.Number, c as Toybox.Lang.Number) as Toybox.Lang.Number { return a ^ b + c; }
    function andProduct(a as Toybox.Lang.Number, b as Toybox.Lang.Number, c as Toybox.Lang.Number) as Toybox.Lang.Number { return a & b * c; }
    function comparison(a as Toybox.Lang.Number, b as Toybox.Lang.Number, c as Toybox.Lang.Number) as Toybox.Lang.Boolean { return a | b == c; }
    function low(a as Toybox.Lang.Number) as Toybox.Lang.Number { System.println("left"); return a; }
    function high(a as Toybox.Lang.Number) as Toybox.Lang.Number { System.println("right"); return a; }
    function flag() as Toybox.Lang.Boolean { System.println("flag"); return true; }
    function run() as Void {
        System.println(Bits.both(12, 10));
        System.println(Bits.either(12, 10));
        System.println(Bits.different(12, 10));
        System.println(Bits.left(1, 31));
        System.println(Bits.right(-8, 1));
        System.println(Bits.right(-2147483648, 31));
        System.println(Bits.complement(0));
        System.println(Bits.complement(-2147483648));
        System.println(Bits.order(1, 2, 2));
        System.println(Bits.product(2, 1, 3));
        System.println(Bits.masks(1, 2, 2));
        System.println(Bits.xors(1, 2, 3));
        System.println(Bits.addOr(1, 1, 2));
        System.println(Bits.xorAdd(3, 1, 2));
        System.println(Bits.andProduct(6, 3, 2));
        System.println(Bits.comparison(1, 2, 3));
        System.println(Bits.low(3) | Bits.high(8));
        System.println(Bits.low(2) << (false || Bits.flag()).hashCode());
        var n = "41".toNumber();
        if (n != null) { System.println(~n & 255); }
        var a = [0];
        a[0] = 1 << 3;
        System.println(a[0]);
        System.println("done");
    }
}
