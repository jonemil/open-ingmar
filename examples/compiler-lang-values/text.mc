using Toybox.System;

module Text {
    function choose(flag as Toybox.Lang.Boolean,
                    left as Toybox.Lang.String,
                    right as Toybox.Lang.String) as Toybox.Lang.String {
        if (flag) { return left; } else { return right; }
    }
    function first() as Toybox.Lang.String {
        System.println("first");
        return "FIRST";
    }
    function second() as Toybox.Lang.String {
        System.println("second");
        return "SECOND";
    }
    function measure(text as Toybox.Lang.String) as Toybox.Lang.Number {
        return text.length();
    }
    function nonempty(text as Toybox.Lang.String) as Toybox.Lang.Boolean {
        return text.length() > 0;
    }
    function show(text as Toybox.Lang.String, flag as Toybox.Lang.Boolean) as Void {
        System.println(text);
        System.println(flag);
    }
}
