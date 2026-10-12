using Toybox.System;

module Numbers {
    function negate(value as Toybox.Lang.Number) as Toybox.Lang.Number {
        return -value;
    }
    function identity(value as Toybox.Lang.Number) as Toybox.Lang.Number {
        return value;
    }
    function guarded(value as Toybox.Lang.Number or Null) as Toybox.Lang.Number {
        if (value == null) { return 0; }
        return -value;
    }
    function effect() as Toybox.Lang.Number {
        System.println("effect");
        return 7;
    }
}
