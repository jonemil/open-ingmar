using Toybox.Application;
using Toybox.Graphics;
using Toybox.System;
using Toybox.WatchUi;

class HelloWorldApp extends Application.AppBase {
    function initialize() {
        AppBase.initialize();
    }

    function onStart(state) {
        System.println("Hello from Monkey C!");
        System.println(probe());
    }

    function getInitialView() {
        return [new HelloWorldView()];
    }
}

// A small, observable function for later instruction-modification experiments.
function probe() {
    return 41;
}

class HelloWorldView extends WatchUi.View {
    function initialize() {
        View.initialize();
    }

    function onUpdate(dc) {
        dc.setColor(Graphics.COLOR_WHITE, Graphics.COLOR_BLACK);
        dc.clear();
        var x = dc.getWidth() / 2;
        var y = dc.getHeight() / 2;
        dc.drawText(x, y - 30, Graphics.FONT_SMALL, "Hello world!",
                    Graphics.TEXT_JUSTIFY_CENTER);
        dc.drawText(x, y + 10, Graphics.FONT_SMALL, probe().toString(),
                    Graphics.TEXT_JUSTIFY_CENTER);
    }
}
