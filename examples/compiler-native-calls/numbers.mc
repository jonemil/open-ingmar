module Numbers {
    function textLength() as Toybox.Lang.Number {
        return "hello".toLower().length();
    }
    function absolute(number as Toybox.Lang.Number) as Toybox.Lang.Number {
        return number.abs();
    }
    function print(number as Toybox.Lang.Number) as Void {
        System.println(number.toString());
    }
    function receiver() as Toybox.Lang.Number {
        System.println("receiver");
        return 41;
    }
    function argument() as Toybox.Lang.Number {
        System.println("argument");
        return 4;
    }
}
