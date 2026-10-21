using Toybox.System;

function start() as Void {
    System.println(Numbers.negate(41));
    System.println(Numbers.negate(-41));
    System.println(Numbers.negate(0));
    System.println(Numbers.negate(-2147483648));
    System.println(-(-2147483648));
    System.println(-(2147483647));
    System.println(-(1 + 2));
    System.println(-(-41));
    System.println((-41).abs());
    System.println(-Numbers.identity(-41).abs());
    System.println(-(1.toNumber()));
    var value = 7;
    System.println(-value);
    value = -value;
    System.println(value);
    System.println(-value + 1);
    if (-value > 0) { System.println("positive"); }
    var nullable = "8".toNumber();
    if (nullable == null) { System.println("unexpected null"); }
    else { System.println(-nullable); }
    System.println(Numbers.guarded(null));
    System.println(-Numbers.effect());
    System.exit();
}
