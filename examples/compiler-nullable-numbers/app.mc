using Toybox.System;

function start() as Void {
    Numbers.showFind("Go bananas!", "bananas");
    Numbers.showFind("Go bananas!", "missing");
    Numbers.showFind("zero", "zero");
    System.println(Numbers.parseOrZero("41tail") + 1);
    System.println(Numbers.parseOrZero("bad"));
    System.println(Numbers.parseOrZero("0"));
    System.println(Numbers.parseOrZero("-7"));
    var number = "41".toNumber();
    if (number == null) {
        System.println("unexpected null");
    } else {
        System.println(number.abs());
        number = "bad".toNumber();
        if (number == null) {
            System.println("became null");
        } else {
            System.println(number);
        }
    }
    var retry = "bad".toNumber();
    while (retry == null) {
        System.println("retry");
        retry = "7".toNumber();
    }
    System.println(retry + 1);
    System.exit();
}
