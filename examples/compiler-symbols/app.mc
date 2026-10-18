using Toybox.Application;
using Toybox.System;
module Symbols {
    function echo(s as Toybox.Lang.Symbol) as Toybox.Lang.Symbol { return s; }
    function run() as Void {
        NullableSymbols.run();
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
module NullableSymbols {
    function value(present as Toybox.Lang.Boolean) as Toybox.Lang.Symbol or Null {return present ? :alpha : null;}
    function reverse(present as Toybox.Lang.Boolean) as Toybox.Lang.Symbol or Null {return present ? null : :beta;}
    function forward(value as Toybox.Lang.Symbol or Null) as Toybox.Lang.Symbol or Null {return value;}
    function name(value as Toybox.Lang.Symbol or Null) as Toybox.Lang.String {
        if (null == value) {return "missing";}
        return value.toString();
    }
    function selector(present as Toybox.Lang.Boolean) as Toybox.Lang.Symbol or Null {return present ? :toString : null;}
    function run() as Void {
        System.println(NullableSymbols.value(false) == null);
        System.println(null != NullableSymbols.value(true));
        System.println(NullableSymbols.reverse(true) == null);
        System.println(NullableSymbols.name(NullableSymbols.forward(:alpha)));
        System.println(NullableSymbols.name(NullableSymbols.forward(null)));
        var value = NullableSymbols.value(true);
        if (value != null) {
            System.println(value.equals(:alpha));
            System.println(value.toNumber() == :alpha.toNumber());
            System.println(value.hashCode() == :alpha.hashCode());
            System.println(value.weak().equals(:alpha));
            System.println(value == :alpha);
        }
        var snapshot = value;
        value = null;
        System.println(NullableSymbols.name(value));
        System.println(NullableSymbols.name(snapshot));
        var visits = 0;
        while (snapshot != null) {
            System.println(snapshot.equals(:alpha));
            snapshot = null;
            visits = visits + 1;
        }
        System.println(visits);
        var selector = NullableSymbols.selector(true);
        if (selector != null) {System.println(1 has selector);}
        System.println(:alpha.equals(NullableSymbols.value(false)));
        System.println(:alpha.equals(NullableSymbols.value(true)));
        var selected = true ? NullableSymbols.reverse(false) : :alpha;
        System.println(NullableSymbols.name(selected));
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Symbols.run(); System.exit(); }
}
