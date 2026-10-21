using Toybox.System;
module Products {
    function left(n as Toybox.Lang.Number) as Toybox.Lang.Number { System.println("left"); return n; }
    function right(n as Toybox.Lang.Number) as Toybox.Lang.Number { System.println("right"); return n; }
    function run() as Void {
        var a = 17;
        var b = 5;
        System.println("products");
        System.println(a * b);
        System.println(a / b);
        System.println(a % b);
        System.println((-a) / b);
        System.println(a / (-b));
        System.println((-a) / (-b));
        System.println((-a) % b);
        System.println(a % (-b));
        System.println((-a) % (-b));
        System.println(2 + 3 * 4);
        System.println((2 + 3) * 4);
        System.println(20 / 3 / 2);
        System.println(20 / (3 / 2));
        System.println(20 % 6 * 3);
        System.println(20 * 6 % 7);
        System.println(-3 * 4);
        System.println(Products.left(7) * Products.right(3));
        System.println(Products.left(17) / Products.right(5));
        System.println(Products.left(17) % Products.right(5));
        var max = 2147483647;
        var min = -2147483648;
        System.println(max * 2);
        System.println(min * (-1));
        System.println(65536 * 65536);
        System.println("done");
    }
}
