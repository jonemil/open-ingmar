using Toybox.System;

function start() as Void {
    if (Flags.mark("skip-and", false) && Flags.mark("bad-and", true)) {
        System.println("unexpected and");
    } else { System.println("and skipped"); }
    if (Flags.mark("skip-or", true) || Flags.mark("bad-or", false)) {
        System.println("or skipped");
    }
    if (Flags.mark("and-left", true) && Flags.mark("and-right", true)) {
        System.println("and both");
    }
    if (Flags.mark("or-left", false) || Flags.mark("or-right", true)) {
        System.println("or both");
    }
    if (true || false && Flags.mark("bad-precedence", true)) {
        System.println("precedence");
    }
    if (!(false || Flags.mark("negated", false))) {
        System.println("inverted");
    }
    var n = "8".toNumber();
    if (!(n == null) && n > 0) { System.println(n); }
    if (n == null || n > 0) { System.println("or guarded"); }
    if (!(n == null)) { System.println(n + 1); }
    Flags.stringGuard(null);
    Flags.stringGuard("ok");
    var counter = 0;
    while (2 > counter && Flags.mark("loop", true)) {
        System.println(counter);
        counter = counter + 1;
    }
    while (!(n == null) && n > 0) {
        System.println(n);
        n = null;
    }
    System.exit();
}
