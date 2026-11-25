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
    function markValue(label as Toybox.Lang.String, value) {
        System.println(label);
        return value;
    }
    function bothValues(left, right, label as Toybox.Lang.String) {
        return left && markValue(label, right);
    }
    function eitherValue(left, right, label as Toybox.Lang.String) {
        return left or markValue(label, right);
    }
    function eitherCondition(left, right, label as Toybox.Lang.String) as Toybox.Lang.Boolean {
        if (left || markValue(label, right)) { return true; }
        return false;
    }
    function truth(value) as Toybox.Lang.Boolean {
        return value ? true : false;
    }
    function count(n as Toybox.Lang.Number) as Toybox.Lang.Number {
        var result = 0;
        if (n) { result += 1; }
        while (n) { result += 2; n -= 1; }
        for (var i = 2; i; i -= 1) { result += 3; }
        do { result += 4; } while (n);
        return result;
    }
    function describe(text as Toybox.Lang.String, flag as Toybox.Lang.Boolean, n as Toybox.Lang.Number) as Toybox.Lang.String {
        return text + flag + n;
    }
    function show(n as Toybox.Lang.Number, flag as Toybox.Lang.Boolean) as Void {
        System.println(n + flag.toString());
    }
}
