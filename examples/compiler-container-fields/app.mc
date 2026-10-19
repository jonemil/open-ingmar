using Toybox.Lang;
using Toybox.System;
class Token {
    var value = 0;
    function initialize(n) {self.value = n;}
    function get() {return self.value;}
}
class Store {
    var object as Lang.Object;
    var maybe as Lang.Object or Null;
    var numbers as Lang.Array<Lang.Number>;
    var mixed as Lang.Array<Lang.Object or Null>;
    var map as Lang.Dictionary<Lang.String,Lang.Object>;
    var numericMap as Lang.Dictionary<Lang.Integer,Lang.Numeric or Null>;
    var bytes as Lang.Array<Lang.ByteArray>;
    var callbacks as Lang.Dictionary<Lang.String,Lang.Method>;
    function initialize() {
        self.object = 1; self.numbers = [3, 1, 2]; self.mixed = [null, 1, "text"];
        self.map = {}; self.numericMap = {}; self.bytes = []; self.callbacks = {};
    }
    function set(o as Lang.Object) as Void {self.object = o;}
    function get() as Lang.Object {return self.object;}
    function read() as Lang.Array<Lang.Number> {return self.numbers;}
    function run() as Void {
        System.println(self.object.toString()); System.println(self.maybe == null);
        var shared = self.numbers; self.numbers[0] += 1; self.numbers.sort(null);
        System.println(shared.toString()); self.numbers = [9];
        System.println(shared.size()); System.println(self.numbers.toString());
        self.mixed.add(true); var snapshot = self.mixed[2]; self.mixed[2] = null;
        if (snapshot != null) {System.println(snapshot.toString());}
        System.println(self.mixed.indexOf(null));
        self.map.put("token", new Token(7)); var token = self.map["token"];
        if (token instanceof Token) {System.println(token.get());}
        self.numericMap.put(1l, 2.5d); var number = self.numericMap[1l];
        if (number != null) {System.println(number + 1);}
        System.println(self.numericMap.values().size());
        var t = new Token(8); self.callbacks.put("get", t.method(:get));
        var callback = self.callbacks["get"];
        if (callback != null) {var result = callback.invoke(); if (result != null) {System.println(result.toString());}}
        self.object = new Token(11); var saved = self.object; self.object = "changed";
        if (saved instanceof Token) {System.println(saved.get());}
        System.println(self.object.toString());
        self.maybe = [1, 2]b; var weak = self.maybe;
        if (weak instanceof Lang.ByteArray) {System.println(weak[0]);}
        self.maybe = null; if (weak != null) {System.println(weak.toString());}
    }
    function holdObject() as Lang.WeakReference {var b = [1]b; self.object = b; return b.weak();}
    function holdArray() as Lang.WeakReference {var b = [2]b; self.bytes.add(b); return b.weak();}
    function holdMap() as Lang.WeakReference {var b = [3]b; self.map.put("held", b); return b.weak();}
    function choose(flag as Lang.Boolean) as Void {
        self.numbers = flag ? [5, 6] : [];
        self.map = flag ? {"number" => 1} : {};
        System.println(self.numbers.size()); System.println(self.map.size());
    }
    function conditional(flag as Lang.Boolean) as Lang.WeakReference {
        var b = [9]b; self.maybe = flag ? b : null; return b.weak();
    }
    function conditionalMap(flag as Lang.Boolean) as Lang.WeakReference {
        var b = [10]b; self.map = flag ? {"held" => b} : {}; return b.weak();
    }
    function lifetime() as Void {
        var weak = self.holdObject(); System.println(weak.stillAlive());
        self.object = 0; System.println(weak.stillAlive());
        weak = self.holdArray(); System.println(weak.stillAlive());
        var snapshot = self.bytes[0]; self.bytes.removeAll(snapshot); System.println(weak.stillAlive());
        snapshot = [4]b; System.println(weak.stillAlive());
        weak = self.holdMap(); System.println(weak.stillAlive());
        var values = self.map.values(); self.map.remove("held"); System.println(weak.stillAlive());
        while (values.size() > 0) {values.removeAll(values[0]);}
        System.println(weak.stillAlive());
        weak = self.conditional(true); System.println(weak.stillAlive());
        self.maybe = null; System.println(weak.stillAlive());
        weak = self.conditional(false); System.println(weak.stillAlive());
        weak = self.conditionalMap(true); System.println(weak.stillAlive());
        self.map = {}; System.println(weak.stillAlive());
        weak = self.conditionalMap(false); System.println(weak.stillAlive());
    }
}
class MinimalConsoleApp extends Toybox.Application.AppBase {
    var store as Lang.Object;
    function initialize() {AppBase.initialize(); self.store = 0;}
    function onStart(state) {
        var a = new Store(); var separate = new Store();
        a.run(); System.println(separate.read().toString()); a.lifetime();
        a.choose(true); a.choose(false);
        self.store = a; var stored = self.store;
        if (stored instanceof Store) {stored.set("stored"); System.println(stored.get().toString());}
        System.exit();
    }
    function getInitialView() {return [new Toybox.WatchUi.View()];}
}
