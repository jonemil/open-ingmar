using Toybox.System;
module Comparisons {
    function left() as Toybox.Lang.Number { System.println("left"); return 3; }
    function right() as Toybox.Lang.Number { System.println("right"); return 5; }
    function parsed(text as Toybox.Lang.String) as Toybox.Lang.Number {
        var value = text.toNumber();
        if (value != null && value <= 17) { return value * 2; }
        return 0;
    }
    function run() as Void {
        var low = -2147483648;
        var high = 2147483647;
        System.println("comparisons");
        System.println(low < high);
        System.println(high < low);
        System.println(3 <= 3);
        System.println(4 <= 3);
        System.println(high >= low);
        System.println(low >= high);
        System.println(3 >= 3);
        System.println(3 != 5);
        System.println(3 != 3);
        System.println(2 + 3 * 4 >= 14);
        System.println(-17 < -3);
        System.println('a' < 'b');
        System.println('a' <= 97);
        System.println(98 >= 'a');
        System.println('a' != 98);
        System.println(true != false);
        System.println(true != true);
        System.println(null != null);
        var text = "same";
        var copied = text;
        System.println(text != copied);
        System.println(text != null);
        System.println(Comparisons.left() < Comparisons.right());
        System.println(Comparisons.left() <= Comparisons.right());
        System.println(Comparisons.left() >= Comparisons.right());
        System.println(Comparisons.left() != Comparisons.right());
        System.println(Comparisons.parsed("17"));
        System.println(Comparisons.parsed("18"));
        System.println(Comparisons.parsed("bad"));
        var count = 0;
        while (count < 3) { count = count + 1; }
        System.println(count);
        System.println("done");
    }
}
