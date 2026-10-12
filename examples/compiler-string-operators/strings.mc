using Toybox.System;

module Strings {
    function identity(value as Toybox.Lang.String) as Toybox.Lang.String {
        return value;
    }
    function number(value as Toybox.Lang.Number or Null) as Toybox.Lang.String {
        return "n=" + value;
    }
    function present(value as Toybox.Lang.String or Null) as Toybox.Lang.String {
        return "[" + value + "]";
    }
    function left() as Toybox.Lang.String {
        System.println("left");
        return "v=";
    }
    function right() as Toybox.Lang.Number {
        System.println("right");
        return 7;
    }
}
