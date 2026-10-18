using Toybox.System;
import Toybox.Lang;
using Toybox.WatchUi;
module SourceTests {
    function read(o as Toybox.Lang.Object or Null) as Toybox.Lang.Number {
        if (o instanceof Probe) { return o.value(); } else { return -1; }
    }
    function asserted(o as Toybox.Lang.Object) as Probe { return o as Probe; }
    function view(o as Toybox.Lang.Object) as Toybox.Lang.Number {
        if (o instanceof LocalView) { return o.value(); } else { return -1; }
    }
    function run() as Void {
        var p = new Probe();
        System.println((p as Lang.Object) instanceof Probe);
        System.println(SourceTests.read(p));
        System.println(SourceTests.read(new NumberOrder()));
        System.println(SourceTests.read(null));
        var erased = p as Lang.Object;
        System.println(SourceTests.asserted(erased).value());
        System.println((erased as Probe).equals(p));
        System.println((3 as Probe) instanceof Lang.Number);
        System.println((null as Probe) instanceof Probe);
        var object = erased;
        if (object instanceof Probe) { System.println(object.value()); object = new NumberOrder(); }
        System.println(object instanceof Probe);
        var reference = erased;
        while (reference instanceof Probe) { System.println(reference.value()); reference = new NumberOrder(); }
        System.println(SourceTests.view(new LocalView()));
        var delegate = new LocalDelegate() as Lang.Object;
        System.println((delegate as LocalDelegate).value());
    }
}
class LocalView extends WatchUi.View {
    function initialize() { View.initialize(); }
    function value() { return 2; }
}
class LocalDelegate extends WatchUi.BehaviorDelegate {
    function initialize() { BehaviorDelegate.initialize(); }
    function value() { return 3; }
}

module ImportedTypes {
    function number(value as Object) as Number { return value as Number + 1; }
    function array(values as Array<String>) as Number { return values.size(); }
    function dictionary(values as Dictionary<String, Number>) as Number { return values.size(); }
    function nullable(value as Object or Null) as String or Null { return value as String or Null; }
    function isChar(value as Object) as Boolean { return value instanceof Char; }
    function run() as Void {
        System.println(ImportedTypes.number(7));
        System.println(ImportedTypes.array(["a", "b"]));
        System.println(ImportedTypes.dictionary({"a" => 1}));
        System.println(ImportedTypes.isChar('a'));
        System.println(ImportedTypes.nullable(null) == null);
        var object = "hello" as Object;
        System.println((object as String).length());
        System.println(object instanceof String);
    }
}
