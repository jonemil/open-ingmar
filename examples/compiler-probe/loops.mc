function choose(value) {
    var remaining = value + 3;
    var total = 0;
    while (remaining > 0) {
        var current = remaining;
        if (current == 2) {
            total = total + 5;
        } else {
            total = total + current;
        }
        System.println(current);
        remaining = remaining - 1;
    }
    System.println(total);
    return 40 + total;
}
