using Toybox.System;

function start() as Void {
    System.println(7.toNumber());
    System.println((0 - 7).abs().format("%03d"));
    System.println((0 - 7).toString());
    System.println(0 > 3.compareTo(7));
    System.println(3.compareTo(3) == 0);
    System.println(7.compareTo(3) > 0);
    System.println(0.compareTo(false) == 0);
    System.println(1.compareTo(true) == 0);
    System.println(7.equals(7));
    System.println(7.equals(8));
    System.println(7.equals("7"));
    System.println(0.equals(false));
    System.println(7.equals(null));
    var number = "7".toNumber();
    System.println(7.equals(number));
    number = null;
    System.println(7.equals(number));
    var text = "7".substring(0, 1);
    System.println(7.equals(text));
    text = null;
    System.println(7.equals(text));
    System.println(Numbers.hash(7) == Numbers.hash(7));
    System.println(Numbers.positiveCompare(7, 3));
    System.println(Numbers.receiver().compareTo(Numbers.argument()) > 0);
    System.exit();
}
