module Numbers {
    function textLength() {
        return "hello".toLower().length();
    }
    function absolute(number) {
        return number.abs();
    }
    function print(number) {
        System.println(number.toString());
    }
    function receiver() {
        System.println("receiver");
        return 41;
    }
    function argument() {
        System.println("argument");
        return 4;
    }
}
