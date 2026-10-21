using Toybox.System;

module Numbers {
    function hash(number as Toybox.Lang.Number) as Toybox.Lang.Number {
        return number.hashCode();
    }
    function positiveCompare(left as Toybox.Lang.Number,
                             right as Toybox.Lang.Number) as Toybox.Lang.Boolean {
        return left.compareTo(right) > 0;
    }
    function receiver() as Toybox.Lang.Number {
        System.println("receiver");
        return 7;
    }
    function argument() as Toybox.Lang.Number {
        System.println("argument");
        return 3;
    }
}
