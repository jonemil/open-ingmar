using Toybox.Application;
using Toybox.System;
module References {
    function expiredArray() as Toybox.Lang.WeakReference { var a = [1]; return a.weak(); }
    function expiredDictionary() as Toybox.Lang.WeakReference { var d = {"one" => 1}; return d.weak(); }
    function echo(w as Toybox.Lang.WeakReference) as Toybox.Lang.WeakReference { return w; }
    function inspect(w as Toybox.Lang.WeakReference) as Void {
        var v = w.get();
        if (v != null) { System.println(v.toString()); } else { System.println("expired"); }
    }
    function object(value as Toybox.Lang.Object) as Toybox.Lang.Object { return value; }
    function choose(flag as Toybox.Lang.Boolean, w as Toybox.Lang.WeakReference) as Toybox.Lang.Object or Null {
        return flag ? w.get() : null;
    }
    function run() as Void {
        var a = [1, 2];
        var w = a.weak();
        var shared = w;
        System.println(w.stillAlive());
        System.println(w.equals(shared));
        System.println(w.equals(a.weak()));
        System.println(w.hashCode() == shared.hashCode());
        System.println(w.toString().equals(w.toString()));
        System.println(References.echo(w).stillAlive());
        var held = w.get();
        System.println(a.equals(held));
        if (held != null) { System.println(References.object(held).equals(a)); }
        a[0] += true ? 6 : 7;
        References.inspect(w);
        a = [3];
        System.println(w.stillAlive());
        held = null;
        System.println(w.stillAlive());
        System.println(w.get() == null);
        References.inspect(w);
        var deadA = References.expiredArray();
        System.println(deadA.stillAlive());
        System.println(deadA.get() == null);
        var deadD = References.expiredDictionary();
        System.println(deadD.stillAlive());
        System.println(deadD.get() == null);
        var d = {"one" => 1};
        var dw = d.weak();
        System.println(dw.stillAlive());
        var got = dw.get();
        if (got != null) { System.println(got.equals(d)); System.println(got.toString()); }
        got = null;
        d = {};
        System.println(dw.stillAlive());
        System.println(dw.get() == null);
        var n = 4;
        var computed = [n, n + 1];
        var cw = computed.weak();
        computed = [0];
        System.println(cw.stillAlive());
        var computedD = {"one" => n};
        var cdw = computedD.weak();
        computedD = {};
        System.println(cdw.stillAlive());
        System.println(References.choose(true, w) == null);
        var ww = w.weak();
        System.println(ww.stillAlive());
        var gotWeak = ww.get();
        if (gotWeak != null) { System.println(gotWeak.equals(w)); }
    }
}
class WeakStore {
    var fixed as Toybox.Lang.WeakReference;
    var maybe as Toybox.Lang.WeakReference or Null = null;
    function initialize(w as Toybox.Lang.WeakReference) { self.fixed = w; }
    function required() as Toybox.Lang.WeakReference { return self.fixed; }
    function set(w as Toybox.Lang.WeakReference or Null) as Void { self.maybe = w; }
    function snapshot() as Toybox.Lang.WeakReference or Null { return self.maybe; }
}
module NullableReferences {
    function choose(w as Toybox.Lang.WeakReference, present as Toybox.Lang.Boolean) as Toybox.Lang.WeakReference or Null { return present ? w : null; }
    function forward(w as Toybox.Lang.WeakReference or Null) as Toybox.Lang.WeakReference or Null { return w; }
    function run() as Void {
        var owner = [10, 20];
        var w = owner.weak();
        var candidate = NullableReferences.choose(w, true);
        var store = new WeakStore(w);
        System.println(store.snapshot() == null);
        store.set(candidate);
        candidate = null;
        var saved = store.snapshot();
        store.set(null);
        if (saved != null) {
            System.println(saved.stillAlive());
            var got = saved.get();
            System.println(got != null);
            owner = [0];
            System.println(saved.stillAlive());
            got = null;
            System.println(saved.stillAlive());
            System.println(saved.get() == null);
        }
        System.println(store.required().stillAlive());
        saved = null;
        System.println(saved == null);
        var none = NullableReferences.choose(w, false);
        System.println(none == null);
        candidate = NullableReferences.forward(w);
        while (candidate != null) {
            System.println(candidate.stillAlive());
            candidate = null;
        }
        System.println(candidate == null);
    }
}
class MinimalConsoleApp extends Application.AppBase {
    var startup as Toybox.Lang.WeakReference or Null;
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        References.run();
        NullableReferences.run();
        var owner = [4];
        self.startup = owner.weak();
        var saved = self.startup;
        self.startup = null;
        if (saved != null) {
            System.println(saved.stillAlive());
            owner = [0];
            System.println(saved.stillAlive());
        }
        System.exit();
    }
}
