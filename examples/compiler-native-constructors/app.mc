using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
using Toybox.WatchUi;
function mark(n as Lang.Number) as Lang.Number { System.println(n); return n; }
class EmptyView extends WatchUi.View {}
class EmptyDelegate extends WatchUi.BehaviorDelegate {}
class RootView extends WatchUi.View {
    var root as Lang.Number = mark(10);
    function initialize(n as Lang.Number) { System.println(100); root = n; }
    function read() as Lang.Number { return root; }
}
class DefaultView extends RootView { var child as Lang.Number = mark(20); }
class InitializedView extends DefaultView {
    var value as Lang.Number;
    function initialize(n as Lang.Number) { RootView.initialize(n); value = root + 1; }
    function total() as Lang.Number { return root + value; }
}
class RootDelegate extends WatchUi.BehaviorDelegate {
    var root as Lang.Number = mark(40);
    function initialize(n as Lang.Number) { System.println(200); root = n; }
    function read() as Lang.Number { return root; }
}
class DefaultDelegate extends RootDelegate { var child as Lang.Number = mark(50); }
class InitializedDelegate extends DefaultDelegate {
    var value as Lang.Number;
    function initialize(n as Lang.Number) {
        BehaviorDelegate.initialize();
        RootDelegate.initialize(n);
        value = root + 1;
    }
    function total() as Lang.Number { return root + value; }
}
class OptionalView extends WatchUi.View {
    function initialize(flag as Lang.Boolean) {
        System.println(500);
        if (flag) { View.initialize(); }
        View.initialize();
    }
}
class OptionalDelegate extends WatchUi.BehaviorDelegate {
    function initialize(flag as Lang.Boolean) {
        System.println(600);
        if (flag) { BehaviorDelegate.initialize(); }
        BehaviorDelegate.initialize();
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        var emptyView = new EmptyView();
        System.println(emptyView instanceof EmptyView);
        var emptyDelegate = new EmptyDelegate();
        System.println(emptyDelegate instanceof EmptyDelegate);
        var view = new DefaultView();
        System.println(view.read());
        var initializedView = new InitializedView(mark(30));
        System.println(initializedView.total());
        var delegate = new DefaultDelegate();
        System.println(delegate.read());
        var initializedDelegate = new InitializedDelegate(mark(60));
        System.println(initializedDelegate.total());
        var optionalView = new OptionalView(true);
        var optionalDelegate = new OptionalDelegate(true);
        var object = new Lang.Object();
        var other = new Toybox.Lang.Object();
        System.println(object.equals(object));
        System.println(object.equals(other));
        System.exit();
    }
}
