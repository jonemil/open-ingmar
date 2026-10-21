using Toybox.System;

function start() as Void {
    System.println(41.weak());
    System.println((0 - 41).weak() + 1);
    System.println("hello".weak());
    System.println("hello".weak().toUpper());
    System.println(true.weak());
    System.println(false.weak());
    if (true.weak()) { System.println("condition"); }
    var number = Values.number(41);
    number = number.weak();
    System.println(number == 41);
    System.println(Values.text("é猫").weak().equals("é猫"));
    System.println(Values.flag(false).weak().equals(false));
    System.println(Values.number(7).weak().weak().toString());
    var nullableNumber = "8".toNumber();
    if (nullableNumber == null) { System.println("unexpected null"); }
    else { System.println(nullableNumber.weak() + 1); }
    var nullableString = "text".substring(0, 4);
    if (nullableString == null) { System.println("unexpected null"); }
    else { System.println(nullableString.weak().toUpper()); }
    System.println(Values.receiver().weak());
    System.exit();
}
