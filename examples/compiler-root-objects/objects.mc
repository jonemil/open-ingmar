using Toybox.Lang as L;
class Counter {
    var count = 1;
    function initialize(n as Toybox.Lang.Number) { self.count = n; }
    function add(n as Toybox.Lang.Number) as Toybox.Lang.Number {
        self.count = self.count + n;
        return self.count;
    }
    function same(o as Toybox.Lang.Object) as Toybox.Lang.Boolean { return self.equals(o); }
}
class Ordering extends L.Object {
    var calls = 0;
    function initialize() {}
    function compare(a as Toybox.Lang.Object, b as Toybox.Lang.Object) as Toybox.Lang.Number {
        self.calls = self.calls + 1;
        return b.toString().compareTo(a.toString());
    }
    function count() as Toybox.Lang.Number { return self.calls; }
}
