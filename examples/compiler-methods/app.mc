using Toybox.Application;
using Toybox.System;
module Methods {
    function bound(a as Toybox.Lang.Array<Toybox.Lang.Number>, s as Toybox.Lang.Symbol) as Toybox.Lang.Method { return a.method(s); }
    function echo(m as Toybox.Lang.Method) as Toybox.Lang.Method { return m; }
    function invokeTwo(m as Toybox.Lang.Method) as Void {
        var result = m.invoke("b", 2);
        System.println(result == null);
    }
    function show(value as Toybox.Lang.Object or Null) as Void {
        if (value != null) { System.println(value.toString()); } else { System.println("null"); }
    }
    function receiver(m as Toybox.Lang.Method) as Toybox.Lang.Method { System.println("receiver"); return m; }
    function argument(n as Toybox.Lang.Number) as Toybox.Lang.Number { System.println("argument"); return n; }
    function run() as Void {
        var a = [1, 2];
        var w = a.weak();
        var size = Methods.bound(a, :size);
        var append = a.method(:add);
        var same = size;
        Methods.show(size.invoke());
        System.println(size.equals(same));
        System.println(size.equals(a.method(:size)));
        System.println(size.hashCode() == same.hashCode());
        System.println(size.toString().equals(size.toString()));
        var mw = size.weak();
        System.println(mw.stillAlive());
        var back = mw.get();
        if (back != null) { System.println(back.equals(size)); }
        back = null;
        a = [0];
        System.println(w.stillAlive());
        append.invoke(3);
        Methods.show(size.invoke());
        var value = Methods.echo(size).invoke();
        if (value != null) { System.println(value.toString()); }
        append = a.method(:add);
        size = a.method(:size);
        same = a.method(:size);
        System.println(w.stillAlive());
        Methods.receiver(append).invoke(Methods.argument(9));
        var d = {"a" => 1};
        var put = d.method(:put);
        Methods.invokeTwo(put);
        System.println(d.size());
        var get = d.method(:get);
        var missing = get.invoke("missing");
        System.println(missing == null);
        var n = 41;
        var abs = n.method(:abs);
        Methods.show(abs.invoke());
        var data = [1, 2];
        var reverse = data.method(:reverse);
        Methods.show(reverse.invoke());
    }
}
module NullableMethods {
    function choose(m as Toybox.Lang.Method, present as Toybox.Lang.Boolean) as Toybox.Lang.Method or Null {
        return present ? m : null;
    }
    function forward(m as Toybox.Lang.Method or Null) as Toybox.Lang.Method or Null { return m; }
    function run() as Void {
        var owner = [7, 8];
        var weak = owner.weak();
        var bound = owner.method(:size);
        var maybe = NullableMethods.choose(bound, true);
        owner = [0];
        bound = owner.method(:size);
        System.println(weak.stillAlive());
        var retained = NullableMethods.forward(maybe);
        maybe = null;
        System.println(weak.stillAlive());
        if (retained != null) { Methods.show(retained.invoke()); }
        retained = null;
        System.println(weak.stillAlive());
        var empty = NullableMethods.choose(bound, false);
        System.println(empty == null);
        var picked = false ? empty : NullableMethods.forward(bound);
        System.println(picked != null && picked.hashCode() == bound.hashCode());
        if (null != picked) {
            var methodWeak = picked.weak();
            System.println(methodWeak.stillAlive());
        }
        while (picked != null) {
            Methods.show(picked.invoke());
            picked = null;
        }
        System.println(picked == null);
        System.println(empty != null && empty.equals(bound));
    }
}
class MethodStore {
    var fixed as Toybox.Lang.Method;
    var maybe as Toybox.Lang.Method or Null = null;
    function initialize(m as Toybox.Lang.Method) { self.fixed = m; }
    function replace(m as Toybox.Lang.Method) as Void { self.fixed = m; }
    function required() as Toybox.Lang.Method { return self.fixed; }
    function set(m as Toybox.Lang.Method or Null) as Void { self.maybe = m; }
    function snapshot() as Toybox.Lang.Method or Null { return self.maybe; }
    function call() as Toybox.Lang.Object or Null { return self.fixed.invoke(); }
}
module MethodFields {
    function run() as Void {
        var owner = [4, 5, 6];
        var weak = owner.weak();
        var bound = owner.method(:size);
        var store = new MethodStore(bound);
        System.println(store.snapshot() == null);
        owner = [0];
        bound = owner.method(:size);
        System.println(weak.stillAlive());
        Methods.show(store.call());
        store.set(store.required());
        store.replace(bound);
        System.println(weak.stillAlive());
        var retained = store.snapshot();
        store.set(null);
        System.println(weak.stillAlive());
        if (retained != null) { Methods.show(retained.invoke()); }
        retained = null;
        System.println(weak.stillAlive());
        Methods.show(store.call());
    }
}
class MinimalConsoleApp extends Application.AppBase {
    var startup as Toybox.Lang.Method or Null;
    function initialize() { AppBase.initialize(); }
    function answer() as Toybox.Lang.Number { return 42; }
    function onStart(state) {
        Methods.run();
        NullableMethods.run();
        MethodFields.run();
        self.startup = method(:answer);
        var saved = self.startup;
        self.startup = null;
        if (saved != null) { Methods.show(saved.invoke()); }
        System.exit();
    }
}
