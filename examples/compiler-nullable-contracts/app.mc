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
        System.println(self.lengthOrZero(retry));
        System.println(Values.booleanOrTrue(Values.boolean(true)));
        System.println(Values.booleanOrTrue(Values.boolean(false)));
        System.println(Values.booleanOrTrue(true));
        System.println(Values.booleanOrTrue(null));
        System.println(Values.charOrDefault(self.passChar(Values.character(true))).toNumber());
        System.println(Values.charOrDefault(self.passChar(null)).toNumber());
        System.println(Values.charOrDefault('Z').toNumber());
        var flag = Values.boolean(false);
        while (flag == null) { flag = Values.boolean(true); }
        if (!flag) { System.println("false is present"); }
        flag = null;
        System.println(flag == null);
        flag = true;
        if (!(flag == null)) { System.println(flag.toString()); }
        var letter = Values.character(false);
        if (letter == null) { letter = 'B'; }
        if (letter != null) { System.println((letter + 1).toNumber()); }
        letter = null;
        System.println(null == letter);
        var copy = self.passChar('C');
        if (copy != null) { System.println(copy.toString()); }
        System.exit();
    }

    function lengthOrZero(value as Toybox.Lang.String or Null)
            as Toybox.Lang.Number {
        if (value == null) { return 0; }
        if (null == value) { return 0; }
        return value.length();
    }

    function passChar(value as Toybox.Lang.Char or Null) as Toybox.Lang.Char or Null {
        return value;
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
