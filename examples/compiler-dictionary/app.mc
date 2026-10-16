using Toybox.Application;
using Toybox.System;
module Dictionaries {
    function same(left as Toybox.Lang.Dictionary, right as Toybox.Lang.Dictionary)
        as Toybox.Lang.Boolean { return left.equals(right); }
    function comparison(value as Toybox.Lang.Dictionary) as Toybox.Lang.Dictionary {
        System.println("argument"); return value;
    }
    function identity() as Void {
        var empty = {};
        var sharedEmpty = empty;
        System.println(empty.equals(empty));
        System.println(empty.equals(sharedEmpty));
        System.println(empty.equals({}));
        System.println(empty.equals(null));
        System.println(empty.equals(1));
        System.println(empty.equals("dictionary"));
        System.println(empty.equals(false));
        System.println(empty.equals('A'));
        System.println(empty.hashCode() == sharedEmpty.hashCode());
        var one = {"one" => 1};
        var equalContents = {"one" => 1};
        System.println(one.equals(equalContents));
        System.println(one.equals(empty));
        var copied = one;
        var hash = one.hashCode();
        one["one"] = 41;
        one.put("two", 2);
        System.println(one.equals(copied));
        System.println(hash == one.hashCode());
        one.remove("one");
        System.println(hash == one.hashCode());
        System.println(Dictionaries.same(one, copied));
        System.println(Dictionaries.same(one, equalContents));
        System.println(one.equals({true => "yes"}));
        System.println(one.equals({'a' => false}));
        System.println(Dictionaries.receiver(one).equals(Dictionaries.comparison(copied)));
        Dictionaries.receiver(one).hashCode();
        Dictionaries.receiver(one).equals(Dictionaries.comparison(equalContents));
        var absent = "bad".toNumber();
        System.println(one.equals(absent));
        var number = "41".toNumber();
        System.println(one.equals(number));
        var text = "hello".substring(0, 1);
        System.println(one.equals(text));
        var boolean = true ? false : null;
        System.println(one.equals(boolean));
        var character = true ? 'A' : null;
        System.println(one.equals(character));
    }
    function flags() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Boolean> {
        return {"one" => false, "two" => true};
    }
    function nullableFlags() as Toybox.Lang.Dictionary<Toybox.Lang.Boolean, Toybox.Lang.Boolean or Null> {
        return {true => false, false => null};
    }
    function characters() as Toybox.Lang.Dictionary<Toybox.Lang.Number, Toybox.Lang.Char> {
        return {1 => 'A', 2 => 'C'};
    }
    function nullableChars() as Toybox.Lang.Dictionary<Toybox.Lang.Char, Toybox.Lang.Char or Null> {
        return {'a' => 'A', 'b' => null};
    }
    function nullFlag() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Boolean or Null> {
        return {"nothing" => null};
    }
    function nullChar() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Char or Null> {
        return {"nothing" => null};
    }
    function flag(value as Toybox.Lang.Boolean or Null) as Toybox.Lang.Boolean or Null {
        System.println("flag"); return value;
    }
    function character(value as Toybox.Lang.Char or Null) as Toybox.Lang.Char or Null {
        System.println("character"); return value;
    }
    function computedFlags(choose as Toybox.Lang.Boolean)
        as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Boolean or Null> {
        return {"first" => choose ? Dictionaries.flag(false) : null,
                "last" => Dictionaries.flag(true)};
    }
    function computedChars(choose as Toybox.Lang.Boolean)
        as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Char or Null> {
        return {"first" => choose ? Dictionaries.character('B') : null,
                "last" => Dictionaries.character('D')};
    }
    function showFlag(value as Toybox.Lang.Boolean or Null) as Void {
        if (value != null) { System.println(value); } else { System.println("null"); }
    }
    function showChar(value as Toybox.Lang.Char or Null) as Void {
        if (value != null) { System.println(value.toNumber()); } else { System.println("null"); }
    }
    function scalarValues() as Void {
        var flags = Dictionaries.flags();
        Dictionaries.showFlag(flags["one"]);
        Dictionaries.showFlag(flags.get("missing"));
        var saved = flags.values();
        var shared = flags;
        shared.put("one", true);
        Dictionaries.showFlag(flags["one"]);
        Dictionaries.showFlag(Dictionaries.flags()["one"]);
        System.println(saved.indexOf(false) >= 0);
        System.println(flags.values().indexOf(false));
        saved[saved.indexOf(false)] = true;
        System.println(flags.size());
        System.println({"one" => false}.toString());
        var nullable = Dictionaries.nullableFlags();
        Dictionaries.showFlag(nullable[true]);
        Dictionaries.showFlag(nullable[false]);
        System.println(nullable.hasKey(false));
        nullable[true] = null;
        nullable.put(false, true);
        Dictionaries.showFlag(nullable.get(false));
        nullable.remove(true);
        System.println(nullable.hasKey(true));
        System.println(nullable.values().indexOf(null));
        System.println(Dictionaries.nullFlag().values().indexOf(null));
        var chars = Dictionaries.characters();
        var before = chars.get(1);
        chars[1] = 'B';
        Dictionaries.showChar(before);
        Dictionaries.showChar(chars[1]);
        Dictionaries.showChar(chars.get(3));
        Dictionaries.showChar(Dictionaries.characters()[1]);
        System.println(chars.values().indexOf('B') >= 0);
        var nullableChars = Dictionaries.nullableChars();
        Dictionaries.showChar(nullableChars['a']);
        Dictionaries.showChar(nullableChars['b']);
        var retained = nullableChars.values();
        nullableChars.put('b', 'B');
        nullableChars['a'] = null;
        System.println(retained.indexOf(null) >= 0);
        Dictionaries.showChar(nullableChars['b']);
        nullableChars.remove('a');
        System.println(nullableChars.hasKey('a'));
        System.println(Dictionaries.nullChar().values().indexOf(null));
        var computedFlags = Dictionaries.computedFlags(true);
        Dictionaries.showFlag(computedFlags["first"]);
        Dictionaries.showFlag(computedFlags["last"]);
        computedFlags = Dictionaries.computedFlags(false);
        Dictionaries.showFlag(computedFlags["first"]);
        var computedChars = Dictionaries.computedChars(true);
        Dictionaries.showChar(computedChars["first"]);
        Dictionaries.showChar(computedChars["last"]);
        computedChars = Dictionaries.computedChars(false);
        Dictionaries.showChar(computedChars["first"]);
        var inferred = {"first" => Dictionaries.flag(false), "second" => null};
        Dictionaries.showFlag(inferred["first"]);
        System.println(inferred.values().indexOf(null) >= 0);
        var inferredChars = {"first" => Dictionaries.character('E'), "second" => null};
        Dictionaries.showChar(inferredChars["first"]);
        System.println(inferredChars.values().indexOf(null) >= 0);
    }
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
        Dictionaries.scalarValues();
        Dictionaries.identity();
        System.println(d.equals(state));
        if (state != null) { System.println(state.hashCode() == state.hashCode()); }
        System.exit();
    }
}
