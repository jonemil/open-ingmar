using Toybox.System;

module Texts {
    function numberText(value as Toybox.Lang.Number or Null)
            as Toybox.Lang.String {
        if (value == null) { return "null"; }
        return value.toString();
    }
    function parse(text as Toybox.Lang.String, base as Toybox.Lang.Number)
            as Toybox.Lang.Number or Null {
        return text.toNumberWithBase(base);
    }
    function receiver() as Toybox.Lang.String {
        System.println("receiver");
        return "a";
    }
    function argument() as Toybox.Lang.String {
        System.println("argument");
        return "b";
    }
}
