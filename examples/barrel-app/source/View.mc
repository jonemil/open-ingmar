using Toybox.Graphics;
using Toybox.WatchUi;

class BarrelView extends WatchUi.View {
    function initialize() {
        View.initialize();
    }

    function onUpdate(dc) {
        dc.setColor(Graphics.COLOR_WHITE, Graphics.COLOR_BLACK);
        dc.clear();
        var x = dc.getWidth() / 2;
        var y = dc.getHeight() / 2;
        dc.drawText(x, y - 30, Graphics.FONT_SMALL, "Barrel example",
                    Graphics.TEXT_JUSTIFY_CENTER);
        dc.drawText(x, y + 10, Graphics.FONT_SMALL, Numbers.answer().toString(),
                    Graphics.TEXT_JUSTIFY_CENTER);
    }
}
