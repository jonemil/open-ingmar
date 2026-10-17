using Toybox.Application;
using Toybox.System;
module Objects {
    function show(m as Toybox.Lang.Method, n as Toybox.Lang.Number) as Void {
        var value = m.invoke(n);
        if (value != null) { System.println(value.toString()); }
    }
    function run() as Void {
        var first = new Counter(3);
        var shared = first;
        var other = new Counter(10);
        System.println(first.add(2));
        System.println(shared.add(1));
        System.println(other.add(0));
        System.println(first.equals(shared));
        System.println(first.equals(other));
        System.println(first.same(shared));
        System.println(first.hashCode() == shared.hashCode());
        System.println(first.toString().equals(shared.toString()));
        var add = first.method(:add);
        first = new Counter(100);
        shared = first;
        Objects.show(add, 4);
        System.println(first.add(0));
        var strings = ["a", "bb", "b"];
        var ordering = new Ordering();
        strings.sort(ordering);
        System.println(strings.toString());
        System.println(ordering.count() > 0);
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Objects.run(); System.exit(); }
}
