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
    function showNumber(value as Toybox.Lang.Number or Null) as Void {
        if (value != null) { System.println(value); }
        else { System.println("null"); }
    }
    function showString(value as Toybox.Lang.String or Null) as Void {
        if (value != null) { System.println(value); }
        else { System.println("null"); }
    }
    function receiver(d as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number>)
        as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number> {
        System.println("receiver"); return d;
    }
    function key() as Toybox.Lang.String { System.println("key"); return "one"; }
    function value() { System.println("value"); return 123; }
    function computedNullable(choose as Toybox.Lang.Boolean, n)
        as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number or Null> {
        return {"one" => choose ? n : null, "two" => n + 1};
    }
    function mutate(d as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number>) {
        System.println("mutate"); d["one"] = 2; return 2;
    }
    function text(d as Toybox.Lang.Dictionary) as Toybox.Lang.String {
        return d.toString();
    }
    function keys(d as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number>)
        as Toybox.Lang.Array<Toybox.Lang.String> { return d.keys(); }
    function values(d as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Number>)
        as Toybox.Lang.Array<Toybox.Lang.Number> { return d.values(); }
    function arrays() as Void {
        var d = {"one" => 1, "two" => 2};
        var keys = Dictionaries.keys(d);
        var values = Dictionaries.values(d);
        System.println(keys.size());
        System.println(keys.indexOf("one") >= 0 && keys.indexOf("two") >= 0);
        System.println(values.indexOf(1) >= 0 && values.indexOf(2) >= 0);
        d["one"] = 41;
        d.put("three", 3);
        System.println(keys.size());
        System.println(values.indexOf(1) >= 0);
        System.println(values.indexOf(41));
        keys[keys.indexOf("one")] = "edited";
        values[values.indexOf(1)] = 7;
        keys.add("extra"); values.add(99);
        System.println(d.size());
        System.println(d.hasKey("one"));
        System.println(d.hasKey("extra"));
        Dictionaries.showNumber(d["one"]);
        System.println(d.keys().indexOf("edited"));
        System.println(d.values().indexOf(7));
        System.println(Dictionaries.makeTyped().keys().size());
        System.println(Dictionaries.makeTyped().values().size());
        var numbers = {1 => "one", 2 => "two"};
        System.println(numbers.keys().indexOf(2) >= 0);
        System.println(numbers.values().indexOf("two") >= 0);
        var flags = {true => "yes", false => null};
        System.println(flags.keys().indexOf(true) >= 0);
        System.println(flags.keys().indexOf(false) >= 0);
        System.println(flags.values().indexOf(null) >= 0);
        var chars = {'a' => 1, 'b' => 2};
        System.println(chars.keys().indexOf('a') >= 0);
        var nullable = Dictionaries.onlyNull();
        System.println(nullable.values().indexOf(null));
        nullable.put("other", 3);
        var retained = nullable.values();
        nullable.remove("nothing");
        System.println(retained.size());
        System.println(retained.indexOf(null) >= 0);
        Dictionaries.receiver(d).keys();
        Dictionaries.receiver(d).values();
    }
    function strings() as Void {
        System.println(Dictionaries.text({}));
        var d = {"one" => 1};
        var before = d.toString();
        d["one"] = 2;
        System.println(before);
        System.println(d.toString());
        System.println({1 => "one"}.toString());
        System.println({true => "yes"}.toString());
        System.println({'a' => 7}.toString());
        System.println(Dictionaries.onlyNull().toString());
        var many = {"one" => 1, "two" => 2}.toString();
        System.println(many.find("one=>1") != null);
        System.println(many.find("two=>2") != null);
        System.println(Dictionaries.receiver(d).toString().length() > 0);
        d.toString();
        System.println(d.size());
    }
    function computed() as Void {
        var duplicate = {Dictionaries.key() => Dictionaries.value(),
                         Dictionaries.key() => Dictionaries.value()};
        System.println(duplicate.size());
        Dictionaries.showNumber(duplicate["one"]);
        var staged = {(true ? Dictionaries.key() : "skip") =>
                      (false ? 0 : Dictionaries.value()),
                      "other" => true ? 7 : 0};
        System.println(staged.size());
        Dictionaries.showNumber(staged["one"]);
        Dictionaries.showNumber(staged["other"]);
        var nullable = Dictionaries.computedNullable(false, 5);
        Dictionaries.showNumber(nullable["one"]);
        Dictionaries.showNumber(nullable["two"]);
        System.println(nullable.size());
        var inferred = {Dictionaries.key() => duplicate.get("missing")};
        System.println(inferred.size());
        System.println(inferred.hasKey("one"));
        Dictionaries.showNumber(inferred["one"]);
        var producer = {"array" => [Dictionaries.value(), 4][0]};
        Dictionaries.showNumber(producer["array"]);
        var state = {"one" => 1};
        var observation = {"before" => state["one"],
                           "after" => Dictionaries.mutate(state)};
        Dictionaries.showNumber(observation["before"]);
        Dictionaries.showNumber(observation["after"]);
        var i = 0;
        while (i < 2) {
            var repeated = {i => i + 1};
            Dictionaries.showNumber(repeated[i]);
            i++;
        }
        var choices = {(false || true) => 1 + 1};
        Dictionaries.showNumber(choices[true]);
    }
    function access() as Void {
        var d = Dictionaries.populated();
        Dictionaries.showNumber(d["one"]);
        Dictionaries.showNumber(d.get("missing"));
        d["one"] = 42;
        var shared = d;
        shared.put("three", 3);
        Dictionaries.showNumber(d.get("one"));
        Dictionaries.showNumber(d["three"]);
        var previous = d["one"];
        d["one"] = 7;
        Dictionaries.showNumber(previous);
        System.println(d.hasKey("one"));
        d.remove("one");
        System.println(d.hasKey("one"));
        Dictionaries.showNumber(d["one"]);
        d.remove("absent");
        System.println(d.size());
        var nullable = Dictionaries.onlyNull();
        System.println(nullable.hasKey("nothing"));
        Dictionaries.showNumber(nullable.get("nothing"));
        nullable["fresh"] = 9;
        nullable.put("nothing", null);
        System.println(nullable.hasKey("nothing"));
        Dictionaries.showNumber(nullable["fresh"]);
        nullable.remove("nothing");
        System.println(nullable.hasKey("nothing"));
        var text = {1 => "one", 2 => "two"};
        text.put(3, "three");
        Dictionaries.showString(text[3]);
        Dictionaries.showString(text.get(4));
        text[2] = "second";
        Dictionaries.showString(text.get(2));
        var flags = {true => "yes", false => null};
        flags.put(true, null);
        System.println(flags.hasKey(true));
        Dictionaries.showString(flags[true]);
        flags[false] = "no";
        Dictionaries.showString(flags[false]);
        var chars = {'a' => 1, 'b' => 2};
        chars['a'] = 10;
        Dictionaries.showNumber(chars.get('a'));
        chars.remove('b');
        System.println(chars.hasKey('b'));
        var first = Dictionaries.populated();
        first["one"] = 99;
        var second = Dictionaries.populated();
        Dictionaries.showNumber(first["one"]);
        Dictionaries.showNumber(second["one"]);
        first.remove("two");
        System.println(second.hasKey("two"));
        Dictionaries.receiver(d)[Dictionaries.key()] = Dictionaries.value();
        Dictionaries.showNumber(d["one"]);
        Dictionaries.receiver(d).put(Dictionaries.key(), Dictionaries.value());
        Dictionaries.showNumber(Dictionaries.receiver(d).get(Dictionaries.key()));
        d[Dictionaries.key()] = true ? Dictionaries.value() : 0;
        Dictionaries.showNumber(d["one"]);
        Dictionaries.showNumber(d[true ? "one" : "missing"]);
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
        Dictionaries.access();
        Dictionaries.computed();
        Dictionaries.strings();
        Dictionaries.arrays();
        System.exit();
    }
}
