module Numbers {
    function identity(value as Toybox.Lang.Number) as Toybox.Lang.Number {
        return value;
    }
    function next(value as Toybox.Lang.Number) as Toybox.Lang.Number {
        return value + 1;
    }
}
