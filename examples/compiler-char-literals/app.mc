using Toybox.System;

function start() as Void {
    System.println('A');
    System.println('é'.toNumber());
    System.println('猫');
    System.println('\n'.toNumber());
    System.println('\r'.toNumber());
    System.println('\t'.toNumber());
    System.println('\''.toNumber());
    System.println('\\'.toNumber());
    System.println('"'.toNumber());
    System.println('A'.toLower());
    System.println('a'.toUpper());
    System.println('A'.equals(65.toChar()));
    System.println('A'.hashCode() == 65.toChar().hashCode());
    System.println(Chars.identity('A'));
    var value = 'A';
    value = 'é';
    System.println(value);
    System.println(Chars.receiver());
    System.println('B'.compareTo('A') > 0);
    System.exit();
}
