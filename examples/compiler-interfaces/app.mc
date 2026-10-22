using Toybox.Lang;
using Toybox.System;

typedef Counter as interface {
    var count as Lang.Number;
    var optional as Lang.Number or Null;
    function add(n as Lang.Number) as Lang.Number;
    function label() as Lang.String;
};

class FirstCounter {
    var count as Lang.Number;
    var optional as Lang.Number or Null;
    function initialize() { self.count = 1; }
    function add(n as Lang.Number) as Lang.Number { self.count += n; return self.count; }
    function label() as Lang.String { return "first"; }
}

class SecondCounter {
    var count as Lang.Number;
    var optional as Lang.Number or Null;
    function initialize() { self.count = 5; }
    function add(n as Lang.Number) as Lang.Number { self.count += n; return self.count; }
    function label() as Lang.String { return "second"; }
}

module InterfaceUse {
    function choose(first as Lang.Boolean) as Counter {
        return first ? new FirstCounter() : new SecondCounter();
    }
    function effect(value as Counter) as Counter {
        System.println("receiver");
        return value;
    }
    function delta() as Lang.Number { System.println("rhs"); return 2; }
    function text(value as interface { function toString() as Lang.String; }) as Lang.String {
        return value.toString();
    }
    function maybe(value as Counter or Null) as Lang.Number {
        if (value == null) { return 0; }
        return value.count;
    }
    function run(first as Lang.Boolean) as Void {
        var counter = InterfaceUse.choose(first);
        System.println(counter.label());
        System.println(counter.count);
        counter.count = 40;
        InterfaceUse.effect(counter).count += InterfaceUse.delta();
        counter.count++;
        --counter.count;
        System.println(counter.add(1));
        var saved = counter.count;
        counter.count = first ? 7 : 8;
        System.println(saved);
        System.println(counter.count);
        counter.optional = 9;
        var optional = counter.optional;
        counter.optional = null;
        if (optional != null) { System.println(optional); }
        System.println(counter.optional == null);
        System.println(InterfaceUse.maybe(counter));
    }
}

class MinimalConsoleApp extends Toybox.Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println("interfaces");
        InterfaceUse.run(true);
        InterfaceUse.run(false);
        System.println(InterfaceUse.text(41));
        System.println(InterfaceUse.maybe(null));
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
