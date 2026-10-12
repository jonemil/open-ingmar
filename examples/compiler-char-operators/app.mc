using Toybox.System;

function start() {
    System.println('A' == 'A');
    System.println('A' == 'B');
    System.println('A' == 65);
    System.println(65 == 'A');
    System.println('A'.equals(65));
    System.println('B' > 'A');
    System.println('A' > 'B');
    System.println('B' > 65);
    System.println(66 > 'A');
    System.println('A' + 1);
    System.println((1 + 'A').toNumber());
    System.println('A' + (0 - 1));
    System.println(('B' + (0 - 1)).toNumber());
    System.println('A' + 'B');
    System.println(('a' + 'b').toUpper());
    System.println('é' > 'A');
    System.println(('é' + 1).toNumber());
    System.println(Chars.advance('A', 2));
    System.println(Chars.join('A', 'B'));
    var value = 'A';
    value = value + 2;
    System.println(value);
    var number = "1".toNumber();
    if (number == null) { System.println("unexpected null"); }
    else { System.println('A' + number); }
    System.println(Chars.left() + Chars.right());
    System.exit();
}
