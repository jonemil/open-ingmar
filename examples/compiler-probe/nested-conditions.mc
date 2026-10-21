function choose(value as Toybox.Lang.Number) as Toybox.Lang.Number {
    var base = 40;
    if (value == 0) {
        var amount = 3;
        System.println(base + amount);
        if (amount > 2) {
            var result = base + amount;
            System.println(result);
            return result;
        } else {
            return base;
        }
    } else {
        var amount = 5;
        System.println(base + amount);
    }
    var result = base + 7;
    System.println(result);
    return result;
}
