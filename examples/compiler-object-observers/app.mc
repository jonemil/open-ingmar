using Toybox.Application;
using Toybox.System;
module Objects {
    function receiver(value as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Array<Toybox.Lang.Number> {
        System.println("receiver"); return value;
    }
    function argument(value as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Array<Toybox.Lang.Number> {
        System.println("argument"); return value;
    }
    function run() as Void {
        var a = [1, 2];
        var shared = a;
        var copy = [1, 2];
        System.println(a.equals(a));
        System.println(a.equals(shared));
        System.println(a.equals(copy));
        System.println(a.equals(null));
        var hash = a.hashCode();
        System.println(hash == shared.hashCode());
        a[0] = 9;
        a.add(3);
        System.println(hash == a.hashCode());
        System.println(a.equals(shared));
        System.println(a.equals(a.slice(null, null)));
        System.println(a.equals(a.reverse()));
        System.println(Objects.receiver(a).equals(Objects.argument(shared)));
        Objects.receiver(a).hashCode();
        Objects.receiver(a).equals(Objects.argument(copy));
        System.println(a.equals([1l]));
        System.println(a.equals([1.25]));
        System.println(a.equals([1.25d]));
        System.println(a.equals(["one"]));
        System.println(a.equals([true]));
        System.println(a.equals(['A']));
        var nullable = true ? 1 : null;
        System.println(a.equals(nullable));
        System.println(a.equals(1l));
        System.println(a.equals(1.25));
        System.println(a.equals(1.25d));
        System.println(a.equals("one"));
        System.println(a.equals(true));
        System.println(a.equals('A'));
        var d = {"one" => 1};
        System.println(a.equals(d));
        System.println(d.equals(a));
        var n = 1;
        var l = 1l;
        var f = 1.25;
        var b = 1.25d;
        var s = "one";
        var t = true;
        var c = 'A';
        System.println(n.equals(a)); System.println(n.equals(d));
        System.println(l.equals(a)); System.println(l.equals(d));
        System.println(f.equals(a)); System.println(f.equals(d));
        System.println(b.equals(a)); System.println(b.equals(d));
        System.println(s.equals(a)); System.println(s.equals(d));
        System.println(t.equals(a)); System.println(t.equals(d));
        System.println(c.equals(a)); System.println(c.equals(d));
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Objects.run(); System.exit(); }
}
