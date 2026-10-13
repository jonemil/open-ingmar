using Toybox.Application;
using Toybox.System;
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { System.println("start"); }
    function getInitialView() {
        System.println("initial");
        var first = new HelloView();
        var second = new HelloView();
        return [first];
    }
    function onStop(state) { System.println("stop"); }
}
