using Toybox.System;

module Flags {
    function identity(value as Toybox.Lang.Boolean) as Toybox.Lang.Boolean {
        return value;
    }
    function receiver() as Toybox.Lang.Boolean {
        System.println("receiver");
        return true;
    }
    function argument() as Toybox.Lang.Number {
        System.println("argument");
        return 0;
    }
}
