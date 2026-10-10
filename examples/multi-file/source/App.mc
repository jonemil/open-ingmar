using Toybox.Application;
using Toybox.System;

class MultiFileApp extends Application.AppBase {
    function initialize() {
        AppBase.initialize();
    }

    function onStart(state) {
        System.println("Hello from multiple files!");
        System.println(Numbers.answer());
    }

    function getInitialView() {
        return [new MultiFileView()];
    }
}
