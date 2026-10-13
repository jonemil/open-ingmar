using Toybox.Application;
using Toybox.System;
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { System.println("start"); }
    function getInitialView() {
        System.println("initial");
        return [new HelloView()];
    }
    function onStop(state) { System.println("stop"); }
}
