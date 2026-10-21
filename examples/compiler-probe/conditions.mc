function choose(value as Toybox.Lang.Number) as Toybox.Lang.Number {
    var adjusted = value + 1;
    if (adjusted == 1) {
        return 47;
    } else {
        return 49;
    }
}
