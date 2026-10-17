using Toybox.Application;
using Toybox.System;
module Symbols {
    function echo(s as Toybox.Lang.Symbol) as Toybox.Lang.Symbol { return s; }
    function run() as Void {
        var a = :alpha;
        var b = :alpha;
        var c = :beta;
        System.println(a == b); System.println(a != c);
        System.println(a.equals(b)); System.println(a.equals(c));
        System.println(a.equals(null)); System.println(a.equals("alpha")); System.println(a.equals(a.toNumber()));
        System.println(a.hashCode() == b.hashCode());
        System.println(a.toNumber() == b.toNumber());
        System.println(a.toNumber() != c.toNumber());
        System.println(a.toString());
        System.println(Symbols.echo(a).equals(a));
        System.println(a.weak().equals(a));
        System.println(:size.toString());
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Symbols.run(); System.exit(); }
}
