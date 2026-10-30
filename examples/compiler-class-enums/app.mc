using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
module Trace {
    const COUNTER as Lang.Array<Lang.Number> = [3];
    function next() as Lang.Number {
        COUNTER[0] += 1;
        System.println(COUNTER[0]);
        return COUNTER[0];
    }
}
class C {
    enum { ZERO, BASE=10, NEXT, GROUPED=(20), AFTER_GROUPED }
    enum Kind { ON=7, NEXT_KIND, CURRENT=Trace.next(), AFTER }
    function initialize() {}
    function value() as Kind { return self.CURRENT; }
    function literal() as Kind { return ON; }
}
class MinimalConsoleApp extends Application.AppBase {
    enum AppKind { APP=42 }
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(C.CURRENT == null);
        System.println(C.GROUPED == null);
        System.println(C.ON);
        System.println(C.ZERO);
        System.println(C.AFTER_GROUPED);
        System.println(MinimalConsoleApp.APP);
        var a = new C();
        System.println(a.value());
        System.println(C.CURRENT);
        System.println(a.literal());
        System.println(a.GROUPED);
        System.println(a.AFTER_GROUPED);
        System.println(C.AFTER);
        var b = new C();
        System.println(a.value());
        System.println(b.value());
        System.println(C.CURRENT);
        System.println(choose(C.ON));
        a.initialize();
        System.println(C.CURRENT);
        System.println(self.APP);
        System.exit();
    }
}
