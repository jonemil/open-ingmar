using Toybox.WatchUi;
using Toybox.System;
using Toybox.Graphics;
class HelloView extends WatchUi.View {
    var count = 7;
    function initialize() {
        View.initialize();
        System.println(self.count);
        self.bump();
        System.println(self.count);
    }
    function bump() as Void { self.count = self.count + 1; }
    function add(n as Toybox.Lang.Number) as Toybox.Lang.Number {
        self.count = self.count + n;
        return self.count;
    }
    function current() as Toybox.Lang.Number { return self.count; }
    function onShow() { System.println("show"); }
    function onUpdate(dc) {
        self.bump();
        System.println(self.count);
        dc.setColor(16777215, 0);
        dc.clear();
        dc.drawText(208, 180, Graphics.FONT_MEDIUM, self.count.toString(), 1);
    }
    function onHide() { System.println("hide"); }
}
