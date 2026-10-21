using Toybox.System;
using Toybox.WatchUi;
class Counter extends WatchUi.View {
    var count = 10;
    function initialize() { View.initialize(); }
    function value() as Toybox.Lang.Number { System.println("field rhs"); self.count = 20; return 3; }
    function flag() as Toybox.Lang.Boolean { System.println("field flag"); self.count = 40; return true; }
    function run() as Void {
        self.count += self.value(); System.println(self.count);
        self.count = 10;
        self.count += (false || self.flag()).hashCode(); System.println(self.count);
        self.count *= 2; self.count >>= 1; System.println(self.count);
    }
}
module Writes {
    function receiver(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Array<Toybox.Lang.Number> { System.println("receiver"); return a; }
    function index() as Toybox.Lang.Number { System.println("index"); return 1; }
    function value(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number { System.println("array rhs"); a.remove(10); return 3; }
    function flag(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Boolean { System.println("array flag"); a.remove(40); a.add(99); return true; }
    function run() as Void {
        var counter = new Counter(); counter.run();
        var a = [10, 20, 30]; var shared = a;
        Writes.receiver(a)[Writes.index()] += Writes.value(a);
        System.println(shared.toString());
        a[0] *= 2; System.println(shared.toString());
        a[1] += (false || Writes.flag(a)).hashCode();
        System.println(shared.toString());
        var chars = ['A', 'B']; var copiedChars = chars;
        chars[0] += 2; chars[1] += -1;
        System.println(copiedChars[0]); System.println(copiedChars[1]);
        System.println("done");
    }
}
