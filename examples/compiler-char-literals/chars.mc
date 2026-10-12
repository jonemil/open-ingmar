using Toybox.System;

module Chars {
    function identity(value as Toybox.Lang.Char) as Toybox.Lang.Char {
        return value;
    }
    function receiver() as Toybox.Lang.Char {
        System.println("receiver");
        return 'A';
    }
}
