using Toybox.Lang;

function choose(value) {
    if (value == 0) {
        return 41;
    } else {
        return 43;
    }
}

function countdown(value) {
    var total = 0;
    while (value > 0) {
        total = total + 2;
        value = value - 1;
    }
    return total;
}

function caught(fail) {
    try {
        if (fail) {
            throw new Lang.Exception();
        }
        return 41;
    } catch (error) {
        return 43;
    }
}
