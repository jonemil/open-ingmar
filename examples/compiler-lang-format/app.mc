using Toybox.Lang;
using Toybox.System;
class Label {
    function initialize() {}
    function toString() as Lang.String { return "label"; }
}
module Formatting {
    function template() as Lang.String { System.println("template"); return "$2$/$1$/$2$"; }
    function item(n as Toybox.Lang.Number) as Toybox.Lang.Number { System.println(n); return n; }
    function typed(a as Lang.Array<Lang.Number>) as Lang.String { return Lang.format("$2$:$1$", a); }
    function run() as Void {
        System.println(Lang.format("$1$:$2$ $3$ $4$", [2, 30, "Sep", 4]));
        System.println(Lang.format("$2$/$1$/$2$", ["a", "b"]));
        System.println(Lang.format("plain", []));
        System.println(Lang.format("$1$ $2$ $3$ $4$", [1l, 2.5, 3.5d, true]));
        System.println(Lang.format("$1$/$2$", ['A', null]));
        System.println(Lang.format("$1$", ["$2$", "recursive"]));
        var numbers = [10, 20]; System.println(Formatting.typed(numbers));
        numbers[0] = 11; System.println(Formatting.typed(numbers));
        var label = new Label();
        System.println(Lang.format("[$1$]", [label]));
        System.println(Lang.format("$1$/$2$", [[1, 2], [3]b]));
        System.println(Lang.format("[$1$]", ["é猫"]));
        System.println(Lang.format("$1$/$2$", [true ? "yes" : "no", false ? 1 : 2]));
        System.println(Lang.format(Formatting.template(), [Formatting.item(1), Formatting.item(2)]));
        Lang.format("discard $1$", [Formatting.item(3)]);
    }
}
class MinimalConsoleApp extends Toybox.Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Formatting.run(); System.exit(); }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
