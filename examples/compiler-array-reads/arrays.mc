using Toybox.System;

module Arrays {
    function first(chars as Toybox.Lang.Array<Toybox.Lang.Char>) as Toybox.Lang.Char {
        return chars[0];
    }
    function effect() as Toybox.Lang.Array<Toybox.Lang.Char> {
        System.println("receiver");
        return "xy".toCharArray();
    }
    function index() as Toybox.Lang.Number {
        System.println("index");
        return 1;
    }
}
