using Toybox.Application;
using Toybox.System;
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { System.println("start"); }
    function getInitialView() {
        System.println("initial");
        var first = Views.make();
        var second = new HelloView();
        var shared = Views.pass(first);
        System.println(Views.update(shared, 3));
        System.println(second.add(1));
        shared = second;
        shared.bump();
        System.println(shared.current());
        System.println(first.current());
        return [self.echo(first)];
    }
    function echo(view as HelloView) as HelloView { return view; }
    function onStop(state) { System.println("stop"); }
}
