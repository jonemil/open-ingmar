import Toybox.Application;
using Toybox.System;

class NullableApp extends AppBase {
    function initialize() { AppBase.initialize(); }

    function onStart(state) {
        System.println(Values.numberOrZero(Values.parse("41")) + 1);
        System.println(Values.numberOrZero(Values.parse("bad")));
        System.println(Values.numberOrZero(7));
        System.println(Values.numberOrZero(null));
        var piece = self.passString(self.choose(true));
        if (piece == null) { System.println("unexpected null"); }
        else { System.println(piece.toUpper()); }
        System.println(Values.textOrFallback(self.passString(self.choose(false))));
        System.println(Values.textOrFallback("scalar"));
        System.println(Values.textOrFallback(null));
        var retry = self.choose(false);
        while (retry == null) {
            System.println("retry");
            retry = self.choose(true);
        }
        System.println(retry.length());
        System.exit();
    }

    function passString(value as Toybox.Lang.String or Null)
            as Toybox.Lang.String or Null {
        return value;
    }

    function choose(present as Toybox.Lang.Boolean)
            as Toybox.Lang.String or Null {
        if (present) { return "hello"; }
        return null;
    }
}
