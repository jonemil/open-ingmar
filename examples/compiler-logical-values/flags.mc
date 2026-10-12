using Toybox.System;

module Flags {
    function mark(label as Toybox.Lang.String, value as Toybox.Lang.Boolean) as Toybox.Lang.Boolean {
        System.println(label);
        return value;
    }
    function text(label as Toybox.Lang.String, value as Toybox.Lang.String) as Toybox.Lang.String {
        System.println(label);
        return value;
    }
    function number(label as Toybox.Lang.String, value as Toybox.Lang.Number) as Toybox.Lang.Number {
        System.println(label);
        return value;
    }
    function both(a as Toybox.Lang.Boolean, b as Toybox.Lang.Boolean) as Toybox.Lang.Boolean {
        return a && b;
    }
    function describe(text as Toybox.Lang.String, flag as Toybox.Lang.Boolean, n as Toybox.Lang.Number) as Toybox.Lang.String {
        return text + flag + n;
    }
    function show(n as Toybox.Lang.Number, flag as Toybox.Lang.Boolean) as Void {
        System.println(n + flag.toString());
    }
}
