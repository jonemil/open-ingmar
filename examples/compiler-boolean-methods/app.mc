using Toybox.System;

function start() {
    System.println(true.toString());
    System.println(false.toString());
    System.println(false.compareTo(false) == 0);
    System.println(true.compareTo(true) == 0);
    System.println(0 > false.compareTo(true));
    System.println(true.compareTo(false) > 0);
    System.println(false.compareTo(0) == 0);
    System.println(true.compareTo(1) == 0);
    System.println(0 > true.compareTo(2));
    System.println(false.compareTo(0 - 1) > 0);
    System.println(true.equals(true));
    System.println(false.equals(false));
    System.println(true.equals(false));
    System.println(true.equals(1));
    System.println(false.equals(0));
    System.println(true.equals("true"));
    System.println(false.equals(null));
    var number = "1".toNumber();
    System.println(true.equals(number));
    number = null;
    System.println(true.equals(number));
    var text = "true".substring(0, 4);
    System.println(true.equals(text));
    text = null;
    System.println(true.equals(text));
    System.println(true.hashCode() == Flags.identity(true).hashCode());
    System.println(false.hashCode() == Flags.identity(false).hashCode());
    var flag = Flags.identity(false);
    flag = true;
    if (flag.equals(true)) { System.println(flag.toString().toUpper()); }
    System.println(Flags.receiver().compareTo(Flags.argument()) > 0);
    System.exit();
}
