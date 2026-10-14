using Toybox.WatchUi;
using Toybox.System;
class CounterDelegate extends WatchUi.BehaviorDelegate {
    var target as CounterView;
    function initialize(view as CounterView) {
        BehaviorDelegate.initialize();
        self.target = view;
    }
    function onSelect() {
        System.println("select");
        System.println(self.target.add(1));
        WatchUi.requestUpdate();
        return true;
    }
}
