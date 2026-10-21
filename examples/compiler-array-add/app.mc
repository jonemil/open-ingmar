using Toybox.System;

module ArrayOps {
    function count(chars as Toybox.Lang.Array<Toybox.Lang.Char>) as Toybox.Lang.Number {
        return chars.size();
    }
    function receiver() as Toybox.Lang.Array<Toybox.Lang.Char> {
        System.println("receiver");
        return "xy".toCharArray();
    }
    function value() as Toybox.Lang.Char {
        System.println("value");
        return 'z';
    }
}

function start() as Void {
    var chars = "ab".toCharArray();
    var shared = chars;
    var returned = chars.add('c');
    System.println(chars.toString());
    System.println(shared.toString());
    System.println(returned.toString());
    System.println(ArrayOps.count(returned));
    System.println(chars.add('d').size());
    System.println(chars.toString());
    var effectful = ArrayOps.receiver().add(ArrayOps.value());
    System.println(effectful.toString());
    System.exit();
}
