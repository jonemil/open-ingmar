using Toybox.WatchUi;
class Ordering extends WatchUi.View {
    var direction = 1;
    var count = 0;
    function initialize(direction as Toybox.Lang.Number) {
        View.initialize();
        self.direction = direction;
    }
    function compare(a as Toybox.Lang.Object, b as Toybox.Lang.Object) as Toybox.Lang.Number {
        self.count = self.count + 1;
        return self.direction * a.toString().compareTo(b.toString());
    }
    function calls() as Toybox.Lang.Number { return self.count; }
}

class TextOrder {
    function initialize() {}
    function compareTo(other as Toybox.Lang.Object) as Toybox.Lang.Number {
        return "b".compareTo(other.toString());
    }
}

class InterfaceHolder {
    var primary as Toybox.Lang.Comparable;
    var requiredOrder as Toybox.Lang.Comparator;
    var maybe as Toybox.Lang.Comparable or Null;
    var selectedOrder as Toybox.Lang.Comparator or Null = null;
    function initialize(value as Toybox.Lang.Comparable) {
        self.primary = value;
        self.requiredOrder = new Ordering(-1);
    }
    function compareValue(other as Toybox.Lang.Object) as Toybox.Lang.Number {
        return self.primary.compareTo(other);
    }
    function replace(value as Toybox.Lang.Comparable) as Void { self.primary = value; }
    function reset() as Void { self.primary = new TextOrder(); }
    function setMaybe(value as Toybox.Lang.Comparable or Null) as Void { self.maybe = value; }
    function compareMaybe(other as Toybox.Lang.Object) as Toybox.Lang.Number {
        var value = self.maybe;
        if (value == null) { return -9; }
        return value.compareTo(other);
    }
    function takeMaybe(other as Toybox.Lang.Object) as Toybox.Lang.Number {
        var value = self.maybe;
        self.maybe = null;
        if (value == null) { return -9; }
        return value.compareTo(other);
    }
    function select(order as Toybox.Lang.Comparator or Null) as Void { self.selectedOrder = order; }
    function sorted() as Toybox.Lang.String {
        var values = ["a", "c", "b"];
        values.sort(self.selectedOrder);
        return values.toString();
    }
    function requiredSorted() as Toybox.Lang.String {
        var values = ["a", "c", "b"];
        values.sort(self.requiredOrder);
        return values.toString();
    }
}
