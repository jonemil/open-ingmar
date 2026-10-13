using Toybox.Application;
using Toybox.System;
class MinimalConsoleApp extends Application.AppBase {
    var controller as ViewController;
    function initialize() {
        AppBase.initialize();
        self.controller = new ViewController(Views.make());
    }
    function onStart(state) { System.println("start"); }
    function getInitialView() {
        System.println("initial");
        System.println(self.controller.update(3));
        return [self.controller.view()];
    }
    function onStop(state) {
        System.println("stop");
        System.println(self.controller.current());
    }
}
