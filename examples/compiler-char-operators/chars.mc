using Toybox.System;

module Chars {
    function advance(value as Toybox.Lang.Char, count as Toybox.Lang.Number)
            as Toybox.Lang.Char {
        return value + count;
    }
    function join(left as Toybox.Lang.Char, right as Toybox.Lang.Char)
            as Toybox.Lang.String {
        return left + right;
    }
    function left() as Toybox.Lang.Char {
        System.println("left");
        return 'A';
    }
    function right() as Toybox.Lang.Number {
        System.println("right");
        return 1;
    }
}
