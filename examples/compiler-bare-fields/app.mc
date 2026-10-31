using Toybox.Application;
using Toybox.System;
var VALUE=100;
const LABEL="global";
const REF="global reference";
module M { function length() as Toybox.Lang.Number { return 99; } }
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        var b=new Box();
        System.println(b.read());
        System.println(b.snapshot);
        System.println(b.CAPTURE);
        System.println(b.label());
        b.change();
        System.println(b.read());
        System.println(b.ITEMS[0]);
        System.println(b.ITEMS.size());
        System.println(b.parameter(10));
        System.println(b.local());
        System.println(b.qualifier());
        System.println(b.globalModule());
        b.forward();
        b.scope();
        System.println(VALUE);
        System.println(LABEL);
        System.exit();
    }
}
