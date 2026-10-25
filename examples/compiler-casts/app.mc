using Toybox.Application;
using Toybox.Lang;
using Toybox.System;

typedef Answer as interface { function answer() as Lang.Number; };
class Counter {
    function answer() as Lang.Number { return 41; }
}
class Other {
    function answer() as Lang.Number { return 42; }
}
module Casts {
    function produce() as Lang.Object {
        System.println("once");
        return [6];
    }
    function run() as Void {
        var erased = [2, 3] as Lang.Object;
        var numbers = erased as Lang.Array<Lang.Number>;
        numbers[0] = 5;
        System.println(numbers[0] + numbers[1]);
        var table = ({"value"=>9} as Lang.Object) as Lang.Dictionary<Lang.String, Lang.Number>;
        var found = table["value"];
        if (found != null) { System.println(found); }
        var original = {:count=>1};
        var options = (original as Lang.Object) as {:count as Lang.Number};
        options[:count] = 11;
        var count = original[:count];
        if (count != null) { System.println(count); }
        var shaped = ([12, "tuple"] as Lang.Object) as [Lang.Number, Lang.String];
        System.println(shaped[0]);
        System.println(shaped[1]);
        var mixed = (13 as Lang.Object) as Lang.Number or Lang.String;
        if (mixed instanceof Lang.Number) { System.println(mixed + 1); }
        var optional = null as Lang.Array<Lang.Number> or Null;
        System.println(optional == null);
        var answer = (new Counter() as Lang.Object) as Answer;
        System.println(answer.answer());
        var comparable = 7 as Lang.Comparable;
        System.println(comparable.compareTo(5) > 0);
        var members = ([new Counter()] as Lang.Object) as Lang.Array<Counter>;
        System.println(members[0].answer());
        var instance = (new Counter() as Lang.Object) as Counter or Other;
        System.println(instance.answer());
        var callback = new Counter().method(:answer) as Method() as Lang.Number;
        System.println(callback.invoke());
        var captured = produce() as Lang.Array<Lang.Number>;
        System.println(captured[0]);
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Casts.run(); System.exit(); }
}
