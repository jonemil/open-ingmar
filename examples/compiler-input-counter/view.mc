using Toybox.WatchUi;
using Toybox.System;
using Toybox.Graphics;
class CounterView extends WatchUi.View {
    var count = 7;
    function initialize() { View.initialize(); }
    function add(amount) {
        self.count = self.count + amount;
        return self.count;
    }
    function onUpdate(dc) {
        System.println("draw");
        System.println(self.count);
        dc.setColor(16777215, 0);
        dc.clear();
        dc.drawText(208, 180, Graphics.FONT_MEDIUM, self.count.toString(), 1);
    }
}
