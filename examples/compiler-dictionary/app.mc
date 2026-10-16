using Toybox.Application;
using Toybox.System;
module Dictionaries {
    function make() as Toybox.Lang.Dictionary { System.println("make"); return {}; }
    function size(d as Toybox.Lang.Dictionary) { System.println("size"); return d.size(); }
    function makeTyped() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number> {
        return {};
    }
    function typedSize(d as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number>) {
        var local = d;
        local = {};
        return local.size();
    }
    function makeNullable() as Toybox.Lang.Dictionary<Toybox.Lang.Number, Toybox.Lang.String or Null> {
        return {};
    }
    function populated() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number> {
        return {"one" => 1, "two" => 2, "one" => 41};
    }
    function onlyNull() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number or Null> {
        return {"nothing" => null};
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        var d = {};
        System.println(d.size()); System.println(d.isEmpty());
        var shared = d;
        System.println(shared.size());
        System.println(Dictionaries.size(Dictionaries.make()));
        System.println(Dictionaries.make().isEmpty());
        var n = 0;
        while (n < 2) { d = {}; System.println(d.isEmpty()); n++; }
        if (state != null) { System.println(state.size()); }
        var typed = Dictionaries.makeTyped();
        System.println(Dictionaries.typedSize(typed));
        typed = {};
        System.println(typed.isEmpty());
        System.println(Dictionaries.typedSize({}));
        System.println(Dictionaries.makeTyped().isEmpty());
        System.println(Dictionaries.makeNullable().isEmpty());
        System.println(Dictionaries.size(typed));
        System.println(Dictionaries.populated().size());
        System.println(Dictionaries.populated().isEmpty());
        var nonempty = Dictionaries.populated();
        nonempty = {"three" => 3};
        System.println(nonempty.size());
        System.println({1 => "same", 2 => "same"}.size());
        System.println({true => "yes", false => null}.size());
        System.println({'a' => -1, 'b' => -2147483648}.size());
        System.println(Dictionaries.onlyNull().size());
        System.exit();
    }
}
