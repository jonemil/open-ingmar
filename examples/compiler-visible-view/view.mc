using Toybox.WatchUi;
using Toybox.System;
using Toybox.Graphics;
class HelloView extends WatchUi.View {
    function initialize() { View.initialize(); }
    function onShow() { System.println("show"); }
    function onUpdate(dc) {
        System.println("update");
        dc.setColor(16777215, 0);
        dc.clear();
        dc.drawText(208, 180, Graphics.FONT_MEDIUM, "Hello from Rust", 1);
    }
    function onHide() { System.println("hide"); }
}
