module Startup {
    function begin() as Void {
        System.println("start");
    }
    function show(left as Toybox.Lang.Number, right as Toybox.Lang.Number) as Void {
        System.println(left - right);
        return;
    }
    function count(limit as Toybox.Lang.Number) as Void {
        var n = limit;
        while (n > 0) {
            if (n == 1) {
                return;
            }
            System.println(n);
            n = n - 1;
        }
        System.println("zero");
    }
}
module Numbers {
    function left() as Toybox.Lang.Number {
        System.println(1);
        return 50;
    }
    function right() as Toybox.Lang.Number {
        System.println(2);
        return 3;
    }
    function answer() as Toybox.Lang.Number {
        System.println(99);
        return 7;
    }
}
