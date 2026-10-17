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
