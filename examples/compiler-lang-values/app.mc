using Toybox.Application;
using Toybox.System;

class LangApp extends Application.AppBase {
    function onStart(state) {
        System.println(self.normalize("Mixed", true));
        System.println(self.normalize("Mixed", false));
        System.println(self.passthrough(false));
        System.println(Text.choose(true, Text.first(), Text.second()));
        System.println(Text.measure(Text.choose(false, "NO", "yes")));
        System.println(Text.nonempty("é猫"));
        System.println(Text.nonempty(""));
        Text.show("printed", true);
        self.discard("discarded");
        System.exit();
    }
    function normalize(text as Toybox.Lang.String,
                       upper as Toybox.Lang.Boolean) as Toybox.Lang.String {
        if (upper) { return text.toUpper(); }
        else { return text.toLower(); }
    }
    function passthrough(value as Toybox.Lang.Boolean) as Toybox.Lang.Boolean {
        return value;
    }
    function discard(text as Toybox.Lang.String) as Toybox.Lang.String {
        System.println(text);
        return text;
    }
}
