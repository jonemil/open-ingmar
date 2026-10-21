using Toybox.System;

function start() as Void {
    var flag = false && Flags.mark("bad-init", true);
    System.println(flag);
    flag = true || Flags.mark("bad-assign", false);
    System.println(flag);
    System.println(Flags.both(true, false));
    System.println(Flags.both(true, true));
    System.println(!(true && false));
    System.println((false || true).equals(true));
    System.println(Flags.text("left", "x") + (false || Flags.mark("right", true)));
    System.println(Flags.text("receiver", "x").equals(false || Flags.mark("native-argument", true)));
    System.println(Flags.describe(
        Flags.text("arg-first", "x"),
        Flags.mark("arg-middle", false) || Flags.mark("arg-right", true),
        Flags.number("arg-last", 7)));
    Flags.show(Flags.number("stmt-first", 3), true && Flags.mark("stmt-second", false));
    System.println((false || true) == (true && false));
    var n = "8".toNumber();
    var valid = !(n == null) && n > 0;
    System.println(valid);
    var counter = 0;
    while (flag) {
        counter = counter + 1;
        flag = 3 > counter && Flags.mark("loop", true);
    }
    System.println(counter);
    System.exit();
}
