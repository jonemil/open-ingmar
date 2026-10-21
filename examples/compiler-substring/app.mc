using Toybox.System;

function start() as Void {
    System.println(Slices.orFallback("Go bananas!", 3, 10));
    System.println(Slices.orFallback("hello", 2, 2).length());
    System.println(Slices.orFallback("hello", 4, 2));
    System.println(Slices.orFallback("hello", 0, 99));
    System.println(Slices.orFallback("hello", 0 - 2, 5));
    var whole = "hello".substring(null, null);
    if (whole == null) {
        System.println("unexpected null");
    } else {
        System.println(whole.toUpper());
        whole = null;
        if (whole == null) { System.println("became null"); }
    }
    var index = "bad".toNumber();
    var tail = "hello".substring(index, 5);
    if (tail == null) { System.println("unexpected null"); }
    else { System.println(tail); }
    var retry = "hello".substring(4, 2);
    retry = null;
    while (retry == null) {
        System.println("retry");
        retry = "hello".substring(1, 4);
    }
    System.println(retry);
    System.exit();
}
