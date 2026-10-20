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

module NullableSourceTests {
    function pass(token as Probe or Null) as Probe or Null { return token; }
    function choose(token as Probe or Null) as Probe or Null {
        if (token == null) { return new Probe(); }
        return null;
    }
    function read(token as Probe or Null) as Number {
        var copy = NullableSourceTests.pass(token);
        if (copy != null) { return copy.value(); }
        return -1;
    }
    function view(token as LocalView or Null) as Number {
        if (token != null) { return token.value(); }
        return -2;
    }
    function delegate(token as LocalDelegate or Null) as Number {
        if (token != null) { return token.value(); }
        return -3;
    }
    function run() as Void {
        System.println(NullableSourceTests.read(new Probe()));
        System.println(NullableSourceTests.read(null));
        System.println(NullableSourceTests.view(new LocalView()));
        System.println(NullableSourceTests.view(null));
        System.println(NullableSourceTests.delegate(new LocalDelegate()));
        System.println(NullableSourceTests.delegate(null));
        var token = NullableSourceTests.choose(null);
        var copy = token;
        if (copy != null) { System.println(copy.value()); }
        copy = null;
        System.println(copy == null);
        token = new Probe();
        while (token != null) { System.println(token.value()); token = null; }
        System.println(NullableSourceTests.choose(new Probe()) == null);
        var holder = new NullableSourceHolder();
        holder.run();
        NullableSourceCasts.run();
        SourceJoins.run();
        CoreLoops.run();
    }
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

class NullableSourceHolder {
    var token as Probe or Null;
    var view as LocalView or Null = null;
    var delegate as LocalDelegate or Null;
    function initialize() {}
    function load() as Probe or Null { return self.token; }
    function snapshot(token as Probe) {
        self.token = token;
        var copy = self.load();
        self.token = null;
        if (copy != null) { return copy.value(); }
        return -1;
    }
    function run() as Void {
        System.println(self.token == null);
        System.println(self.view == null);
        System.println(self.delegate == null);
        self.token = new Probe();
        self.view = new LocalView();
        self.delegate = new LocalDelegate();
        var token = self.token;
        var view = self.view;
        var delegate = self.delegate;
        self.token = null;
        self.view = null;
        self.delegate = null;
        if (token != null) { System.println(token.value()); }
        if (view != null) { System.println(view.value()); }
        if (delegate != null) { System.println(delegate.value()); }
        System.println(self.snapshot(new Probe()));
        var again = self.load();
        System.println(again == null);
    }
}

module NullableSourceCasts {
    function effect() as Object or Null {
        System.println("source-cast-effect");
        return new Probe();
    }
    function cast(value as Object or Null) as Probe or Null {
        return value as Probe or Null;
    }
    function run() as Void {
        var token = NullableSourceCasts.cast(new Probe());
        if (token != null) { System.println(token.value()); }
        token = NullableSourceCasts.cast(null);
        System.println(token == null);
        var direct = null as Probe or Null;
        System.println(direct == null);
        var view = new LocalView() as Object;
        var recovered = view as LocalView or Null;
        if (recovered != null) { System.println(recovered.value()); }
        var delegate = new LocalDelegate() as Object;
        var local = delegate as LocalDelegate or Null;
        if (local != null) { System.println(local.value()); }
        var effect = NullableSourceCasts.effect() as Probe or Null;
        if (effect != null) { System.println(effect.value()); }
        System.println((3 as Probe or Null) instanceof Number);
        System.println((new LocalView() as Probe or Null) instanceof LocalView);
        System.println(NullableSourceCasts.cast(null) == null or false);
        var asserted = (new Probe() as Probe or Null) as Probe;
        System.println(asserted.value());
    }
}

module SourceJoins {
    function choose(flag as Boolean) as Probe or Null {
        return flag ? new Probe() : null;
    }
    function fromObject(value as Object) as Probe or Null {
        return value instanceof Probe ? value : null;
    }
    function left() as Probe { System.println("source-join-left"); return new Probe(); }
    function right() as Probe { System.println("source-join-right"); return new Probe(); }
    function run() as Void {
        var same = true ? new Probe() : new Probe();
        System.println(same.value());
        var maybe = SourceJoins.choose(false);
        System.println(maybe == null);
        maybe = SourceJoins.choose(true);
        if (maybe != null) { System.println(maybe.value()); }
        var nested = false ? null : (true ? new Probe() : null);
        if (nested != null) { System.println(nested.value()); }
        var view = true ? new LocalView() : null;
        if (view != null) { System.println(view.value()); }
        var delegate = false ? null : new LocalDelegate();
        if (delegate != null) { System.println(delegate.value()); }
        var first = true ? SourceJoins.left() : SourceJoins.right();
        System.println(first.value());
        var second = false ? SourceJoins.left() : SourceJoins.right();
        System.println(second.value());
        var guarded = SourceJoins.fromObject(new Probe());
        if (guarded != null) { System.println(guarded.value()); }
        System.println(SourceJoins.fromObject(new NumberOrder()) == null);
    }
}

module CoreLoops {
    function begin() { System.println("loop-init"); return 0; }
    function test(n as Number) as Boolean { System.println("loop-test"); return n < 2; }
    function step(n as Number) { System.println("loop-step"); return n + 1; }
    function falseMark() as Boolean { System.println("bad-do-test"); return false; }
    function returnDo() { do { return 41; } while(CoreLoops.falseMark()); }
    function returnFor() { for(;;) { return 42; } }
    function proof(value as Probe or Null) as Void {
        if (value != null) {
            do { break; } while(CoreLoops.falseMark());
            System.println(value.value());
        }
    }
    function run() as Void {
        var sum = 0;
        for (var i = 0; i < 6; i++) {
            if (i == 1) { continue; }
            if (i == 5) { break; }
            sum += i;
        }
        System.println(sum);
        var n = 0;
        do { n++; if (n < 3) { continue; } break; } while(true);
        System.println(n);
        var once = 0;
        do { once++; } while(false);
        System.println(once);
        for (var trace = CoreLoops.begin(); CoreLoops.test(trace); trace = CoreLoops.step(trace)) {
            System.println(trace);
        }
        for (var j = 0; j < 2; j = CoreLoops.step(j)) { break; }
        var j = 7;
        System.println(j);
        var k = 0;
        for (; k < 2;) { k++; }
        System.println(k);
        for (;;) { break; }
        System.println("for-empty");
        var count = 0;
        for (var outer = 0; outer < 3; outer++) {
            var inner = 0;
            do { inner++; if (inner == 2) { continue; } count++; } while(inner < 3);
        }
        System.println(count);
        System.println(CoreLoops.returnDo());
        System.println(CoreLoops.returnFor());
        var nullable = "8".toNumber();
        do {
            if (nullable == null) { break; }
            System.println(nullable);
            nullable = null;
        } while(nullable != null);
        CoreLoops.proof(new Probe());
    }
}
