using Toybox.System;

function start() {
    System.println(65.toChar());
    System.println(65.toChar().toNumber());
    System.println(65.toChar().toLower());
    System.println(97.toChar().toUpper().toString());
    System.println(Chars.fromText("233").toString());
    System.println(Chars.fromText("29483").toString());
    System.println(Chars.fromText("128512").toNumber());
    System.println(0.toChar().toNumber());
    System.println((0 - 1).toChar().toNumber());
    System.println(Chars.fromText("55296").toNumber());
    System.println(Chars.fromText("1114112").toNumber());
    System.println(65.toChar().equals(65.toChar()));
    System.println(65.toChar().equals(66.toChar()));
    System.println(65.toChar().equals(65));
    System.println(65.toChar().equals("A"));
    System.println(1.toChar().equals(true));
    System.println(65.toChar().equals(null));
    System.println(65.equals(65.toChar()));
    System.println("A".equals(65.toChar()));
    System.println(true.equals(1.toChar()));
    System.println(65.toChar().hashCode() == Chars.identity(65.toChar()).hashCode());
    System.println(65.toChar().compareTo(65) == 0);
    System.println(0 > 65.toChar().compareTo(66.toChar()));
    System.println(1.toChar().compareTo(true) == 0);
    System.println(65.compareTo(65.toChar()) == 0);
    System.println(true.compareTo(1.toChar()) == 0);
    var value = Chars.identity(65.toChar());
    value = 66.toChar();
    System.println(value);
    var number = "65".toNumber();
    if (number == null) { System.println("unexpected null"); }
    else { System.println(number.toChar().toLower()); }
    System.println(0 > Chars.receiver().compareTo(Chars.argument()));
    System.exit();
}
