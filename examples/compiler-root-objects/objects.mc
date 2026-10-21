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
    function compare(a as Toybox.Lang.Object, b as Toybox.Lang.Object) as Toybox.Lang.Number {
        self.calls = self.calls + 1;
        return b.toString().compareTo(a.toString());
    }
    function count() as Toybox.Lang.Number { return self.calls; }
}

class EmptyToken {}

class DefaultFields {
    var count = 7;
    var maybe as Toybox.Lang.String or Null;
    function value() as Toybox.Lang.Number { return self.count; }
    function empty() as Toybox.Lang.Boolean { return self.maybe == null; }
    function step() as Toybox.Lang.Number {
        self.count += 1;
        return self.count;
    }
}

class CounterBox {
    var target as Counter;
    function initialize(c as Counter) { self.target = c; }
    function current() as Counter { return self.target; }
    function swap(c as Counter) as Counter {
        var previous = self.current();
        self.target = c;
        return previous;
    }
}
