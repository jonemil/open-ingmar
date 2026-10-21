using Toybox.System;

function start() as Void {
    System.println(!true);
    System.println(!false);
    System.println(Flags.invert(true));
    System.println(Flags.invert(false));
    System.println(!!true);
    System.println(!true.equals(1));
    System.println((!true).equals(1));
    System.println(!(1 > 2));
    System.println(!(1 == 1));
    var flag = true;
    flag = !flag;
    System.println(flag);
    if (!flag) { System.println("inverted"); }
    var n = "8".toNumber();
    if (!(n == null)) { System.println("present"); }
    System.println(!Flags.effect());
    System.exit();
}
