function choose(value as Toybox.Lang.Number) as Toybox.Lang.Number {
    var remaining = value + 4;
    var total = 0;
    while (remaining > 0) {
        remaining = remaining - 1;
        if (remaining == 2) {
            continue;
        }
        var inner = 3;
        while (inner > 0) {
            inner = inner - 1;
            if (inner == 1) {
                continue;
            }
            if (remaining == 1) {
                break;
            }
            total = total + 1;
        }
        if (remaining == 0) {
            break;
        }
    }
    System.println(total);
    return 40 + total;
}
