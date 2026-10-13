using Toybox.Application;
using Toybox.System;
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { System.println("start"); }
    function getInitialView() {
        System.println("initial");
        var first = new HelloView();
        var second = new HelloView();
        var shared = first;
        System.println(shared.add(3));
        System.println(second.add(1));
        shared = second;
        shared.bump();
        System.println(shared.current());
        System.println(first.current());
        return [first];
    }
    function onStop(state) { System.println("stop"); }
}
