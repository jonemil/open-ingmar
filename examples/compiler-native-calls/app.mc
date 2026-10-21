function start() as Void {
    var number = 41;
    System.println(number.toString());
    var negative = 0 - 1;
    var text = negative.toString();
    System.println(text);
    Numbers.print(0);
    System.println((number + 1).format("%03d"));
    var pattern = "%02d";
    System.println(7.format(pattern));
    System.println((Numbers.receiver() + Numbers.argument()).format("%04d"));
    var magnitude = Numbers.absolute(negative - 40);
    System.println(magnitude + 1);
    System.println((negative - 40).abs().format("%03d"));
    System.println(0.abs());
    System.println(7.abs().abs());
    var greeting = "Hello Monkey C!";
    System.println(greeting.length());
    System.println(greeting.toLower());
    System.println(greeting.toUpper());
    System.println(greeting.toString());
    System.println(greeting);
    System.println("".length());
    System.println("é猫".length());
    System.println(number.toString().toUpper().length().format("%03d"));
    System.println(Numbers.textLength() + 1);
}
