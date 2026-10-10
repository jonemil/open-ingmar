using Toybox.Application;
using Toybox.System;

class BarrelApp extends Application.AppBase {
    function initialize() {
        AppBase.initialize();
    }

    function onStart(state) {
        System.println("Hello from a barrel!");
        System.println(Numbers.answer());
    }

    function getInitialView() {
        return [new BarrelView()];
    }
}
