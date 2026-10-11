function start() {
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
}
