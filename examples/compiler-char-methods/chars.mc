using Toybox.System;

module Chars {
    function identity(value as Toybox.Lang.Char) as Toybox.Lang.Char {
        return value;
    }
    function fromText(text as Toybox.Lang.String) as Toybox.Lang.Char {
        var value = text.toNumber();
        if (value == null) { return 0.toChar(); }
        return value.toChar();
    }
    function receiver() as Toybox.Lang.Char {
        System.println("receiver");
        return 65.toChar();
    }
    function argument() as Toybox.Lang.Char {
        System.println("argument");
        return 66.toChar();
    }
}
