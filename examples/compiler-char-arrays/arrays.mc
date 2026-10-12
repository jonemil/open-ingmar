using Toybox.System;

module Arrays {
    function count(chars as Toybox.Lang.Array<Toybox.Lang.Char>) as Toybox.Lang.Number {
        return chars.size();
    }
    function convert(text as Toybox.Lang.String) as Toybox.Lang.Array<Toybox.Lang.Char> {
        return text.toCharArray();
    }
    function effect() as Toybox.Lang.String {
        System.println("effect");
        return "abc";
    }
}
