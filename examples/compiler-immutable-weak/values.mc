using Toybox.System;

module Values {
    function number(value as Toybox.Lang.Number) as Toybox.Lang.Number {
        return value.weak();
    }
    function text(value as Toybox.Lang.String) as Toybox.Lang.String {
        return value.weak();
    }
    function flag(value as Toybox.Lang.Boolean) as Toybox.Lang.Boolean {
        return value.weak();
    }
    function receiver() as Toybox.Lang.Number {
        System.println("receiver");
        return 3;
    }
}
