using Toybox.Application;
using Toybox.System;

class ModuleNamesApp extends Application.AppBase {
    function initialize() {
        AppBase.initialize();
    }

    function onStart(state) {
        System.println("Hello from named modules!");
        System.println(Left.answer());
        System.println(Right.answer());
        System.println(Left.answer() + Right.answer());
    }

    function getInitialView() {
        return [new ModuleNamesView()];
    }
}
