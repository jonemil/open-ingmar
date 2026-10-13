using Toybox.Application;
using Toybox.WatchUi;
using Toybox.System;
class MinimalConsoleApp extends Application.AppBase {
    function initialize() {
        AppBase.initialize();
    }
    function onStart(state) {
        System.println("start");
    }
    function makeView() as Toybox.WatchUi.View {
        System.println("make");
        return new WatchUi.View();
    }
    function getInitialView() {
        System.println("initial");
        return [self.makeView()];
    }
}
