using Toybox.Lang;

function choose(value as Toybox.Lang.Number) as Toybox.Lang.Number {
    if (value == 0) {
        return 41;
    } else {
        return 43;
    }
}

function countdown(value as Toybox.Lang.Number) as Toybox.Lang.Number {
    var total = 0;
    while (value > 0) {
        total = total + 2;
        value = value - 1;
    }
    return total;
}

function caught(fail as Toybox.Lang.Number) as Toybox.Lang.Number {
    try {
        if (fail) {
            throw new Lang.Exception();
        }
        return 41;
    } catch (error) {
        return 43;
    }
}
