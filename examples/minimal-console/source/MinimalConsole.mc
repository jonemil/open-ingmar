using Toybox.Application;
using Toybox.System;
using Toybox.WatchUi;

// The function we want to disassemble and reassemble first.
function hello() {
    System.println("Hello!");
}

// A watch app needs lifecycle plumbing even for a console-only specimen.
class MinimalConsoleApp extends Application.AppBase {
    function initialize() {
        AppBase.initialize();
    }

    function onStart(state) {
        hello();
    }

    function getInitialView() {
        return [new WatchUi.View()];
    }
}
