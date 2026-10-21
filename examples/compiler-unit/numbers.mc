module Numbers {
    function difference(first as Toybox.Lang.Number, second as Toybox.Lang.Number) as Toybox.Lang.Number {
        var adjusted = first + 3;
        return adjusted - second;
    }
    function left() as Toybox.Lang.Number {
        System.println(1);
        return 40 + 5;
    }
    function right() as Toybox.Lang.Number {
        System.println(2);
        return 2;
    }
}
