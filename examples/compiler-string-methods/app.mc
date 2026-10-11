using Toybox.System;

function start() {
    System.println("Hello".toLower());
    System.println("Hello".toUpper());
    System.println("Hello".toString());
    System.println("".length());
    System.println(0 > "a".compareTo("b"));
    System.println("same".compareTo("same") == 0);
    System.println("b".compareTo("a") > 0);
    System.println("same".equals("same"));
    System.println("same".equals("Same"));
    System.println("7".equals(7));
    System.println("false".equals(false));
    System.println("same".equals(null));
    var text = "same".substring(0, 4);
    System.println("same".equals(text));
    text = null;
    System.println("same".equals(text));
    var number = "7".toNumber();
    System.println("7".equals(number));
    number = null;
    System.println("7".equals(number));
    System.println("same".hashCode() == "same".toString().hashCode());
    System.println(Texts.numberText("hello".find("ll")));
    System.println(Texts.numberText("hello".find("missing")));
    System.println(Texts.numberText("41tail".toNumber()));
    System.println(Texts.numberText(Texts.parse("101", 2)));
    System.println(Texts.numberText(Texts.parse("FF", 16)));
    System.println(Texts.numberText(Texts.parse("z", 36)));
    System.println(Texts.numberText(Texts.parse("0x10", 0)));
    System.println(Texts.numberText(Texts.parse("010", 0)));
    System.println(Texts.numberText(Texts.parse("-10", 16)));
    System.println(Texts.numberText(Texts.parse("bad", 10)));
    System.println("é猫z".length());
    System.println(Texts.numberText("é猫z".find("猫")));
    var piece = "é猫z".substring(1, 2);
    if (piece == null) { System.println("unexpected null"); }
    else { System.println(piece); }
    System.println(0 > Texts.receiver().compareTo(Texts.argument()));
    System.exit();
}
