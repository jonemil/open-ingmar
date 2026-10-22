using Toybox.Lang;
using Toybox.System;

typedef Unary as Method(n as Lang.Number) as Lang.Number;
typedef Sink as Method(n as Lang.Number) as Void;
typedef Factory as Method() as Token;
typedef Provider as Method() as Unary;

class Token {
    function number() as Lang.Number { return 41; }
}

class Worker {
    function step(n as Lang.Number) as Lang.Number { return n + 1; }
    function sink(n as Lang.Number) as Void { System.println("sink"); System.println(n); }
    function factory() as Token { return new Token(); }
    function callback() as Unary { return method(:step); }
    function provide() as Unary { return method(:step); }
}

class Store {
    var callback as Unary;
    function initialize(callback as Unary) { self.callback = callback; }
    function run() as Lang.Number { return self.callback.invoke(41); }
}

module Callbacks {
    function apply(callback as Unary, n as Lang.Number) as Lang.Number { return callback.invoke(n); }
    function maybe(callback as Unary) as Unary or Null { return callback; }
    function send(callback as Sink, n as Lang.Number) as Void { callback.invoke(n); }
    function create(callback as Factory) as Token { return callback.invoke(); }
    function erased(callback as Lang.Method) as Lang.Number { return Callbacks.apply(callback, 41); }
    function provider(callback as Provider) as Lang.Number { return callback.invoke().invoke(8); }
    function array(callback as Unary) as Lang.Array<Unary or Null> { return [callback, null]; }
    function dictionary(callback as Unary) as Lang.Dictionary<Unary, Unary or Null> { return {callback => callback}; }
}

class MinimalConsoleApp extends Toybox.Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println("callbacks");
        var worker = new Worker();
        var callback = worker.callback();
        System.println(Callbacks.apply(worker.method(:step), 41));
        System.println(callback.invoke(1));
        var maybe = Callbacks.maybe(callback);
        var saved = maybe;
        maybe = null;
        if (saved != null) { System.println(saved.invoke(2)); }
        System.println(new Store(callback).run());
        Callbacks.send(worker.method(:sink), 7);
        System.println(Callbacks.create(worker.method(:factory)).number());
        var length = "garmin".method(:length);
        System.println(length.invoke());
        var values = Callbacks.array(callback);
        var item = values[0];
        values[0] = null;
        if (item != null) { System.println(item.invoke(3)); }
        var dictionary = Callbacks.dictionary(callback);
        var entry = dictionary[callback];
        dictionary[callback] = null;
        if (entry != null) { System.println(entry.invoke(4)); }
        System.println(dictionary.keys()[0].invoke(6));
        System.println(Callbacks.erased(callback));
        System.println(Callbacks.provider(worker.method(:provide)));
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
