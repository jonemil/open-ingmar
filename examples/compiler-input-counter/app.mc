using Toybox.Application;
using Toybox.System;
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { System.println("start"); }
    function getInitialView() {
        var view = new CounterView();
        return [view, new CounterDelegate(view)];
    }
    function onStop(state) { System.println("stop"); }
}
