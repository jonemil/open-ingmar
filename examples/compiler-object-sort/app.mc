using Toybox.Lang;
using Toybox.System;
class TextOrder {
    var count = 0;
    var direction = 1;
    function initialize(n) { self.direction = n; }
    function compare(a as Lang.Object, b as Lang.Object) as Lang.Number {
        self.count++;
        return self.direction * a.toString().compareTo(b.toString());
    }
    function called() as Lang.Boolean { return self.count > 0; }
}
class Label {
    var value = 0;
    function initialize(n) { self.value = n; }
    function toString() as Lang.String { return self.value.toString(); }
}
module ObjectSort {
    function numeric() as Lang.Array<Lang.Object> { return [3, 1l, 2.0, 0.0d]; }
    function strings() as Lang.Array<Lang.Object> { return ["bb", "a", "aa", "b", "a"]; }
    function mixed() as Lang.Array<Lang.Object> { return ["text", 2, true, 'A']; }
    function labels() as Lang.Array<Lang.Object> { return [new Label(3), new Label(1), new Label(2)]; }
    function empty() as Lang.Array<Lang.Object> { return []; }
    function one() as Lang.Array<Lang.Object> { return [new Label(7)]; }
    function array(a as Lang.Array<Lang.Object>) as Lang.Array<Lang.Object> { System.println("array"); return a; }
    function comparator(c as TextOrder) as TextOrder { System.println("comparator"); return c; }
    function sort(a as Lang.Array<Lang.Object>, c as TextOrder) as Void { a.sort(c); }
    function symbols() as Void {
        var a = [:z, :a]; var cmp = new TextOrder(1);
        a.sort(cmp);
        System.println(a[0].toString().compareTo(a[1].toString()) <= 0);
        System.println(a.indexOf(:z) >= 0); System.println(a.indexOf(:a) >= 0);
    }
    function refs() as Void {
        var bytes = [[2]b, [1]b, [3]b]; var saved = bytes[0];
        var cmp = new TextOrder(1); bytes.sort(cmp);
        System.println(bytes.toString()); saved[0] = 9; System.println(bytes[1][0]);
        var a = new Label(1); var b = new Label(2);
        var first = a.method(:toString); var second = b.method(:toString);
        var methods = [first, second]; methods.sort(cmp);
        System.println(methods.size()); System.println(methods.indexOf(first) >= 0);
        var wa = [1]; var wb = [2]; var weak = [wa.weak(), wb.weak()]; weak.sort(cmp);
        System.println(weak[0].stillAlive()); System.println(weak[1].stillAlive());
        wa = [3]; wb = [4]; System.println(weak[0].stillAlive()); System.println(weak[1].stillAlive());
        var views = [new Toybox.WatchUi.View(), new Toybox.WatchUi.View()]; views.sort(cmp);
        System.println(views.size()); System.println(cmp.called());
    }
    function run() as Void {
        var numeric = ObjectSort.numeric(); var shared = numeric; numeric.sort(null);
        System.println(shared.toString()); System.println(numeric[0] instanceof Lang.Double);
        System.println(numeric[1] instanceof Lang.Long); System.println(numeric[2] instanceof Lang.Float);
        var strings = ObjectSort.strings(); strings.sort(null); System.println(strings.toString());
        var ascending = new TextOrder(1); var descending = new TextOrder(-1);
        var mixed = ObjectSort.mixed(); mixed.sort(ascending); System.println(mixed.toString());
        var labels = ObjectSort.labels(); var before = labels.slice(0, 3);
        ObjectSort.array(labels).sort(ObjectSort.comparator(descending));
        System.println(labels.toString()); System.println(before.toString()); System.println(descending.called());
        ObjectSort.sort(labels, ascending); System.println(labels.toString());
        var empty = ObjectSort.empty(); empty.sort(null); empty.sort(ascending); System.println(empty.size());
        var one = ObjectSort.one();
        one.sort(null); System.println(one.toString());
        ObjectSort.symbols(); ObjectSort.refs();
    }
}
class MinimalConsoleApp extends Toybox.Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { ObjectSort.run(); System.exit(); }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
