using Toybox.System;

module Flags {
    function invert(value as Toybox.Lang.Boolean) as Toybox.Lang.Boolean {
        return !value;
    }
    function effect() as Toybox.Lang.Boolean {
        System.println("effect");
        return true;
    }
}
