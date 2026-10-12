using Toybox.System;

module Flags {
    function mark(label as Toybox.Lang.String, value as Toybox.Lang.Boolean) as Toybox.Lang.Boolean {
        System.println(label);
        return value;
    }
    function stringGuard(text as Toybox.Lang.String or Null) as Void {
        if (!(text == null) && text.length() > 0) { System.println(text); }
        else { System.println("empty"); }
    }
}
