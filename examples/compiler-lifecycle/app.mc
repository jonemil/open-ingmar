using Toybox.Application as App;
using Toybox.System as Sys;

class StartupApp extends App.AppBase {
    function initialize() {
        AppBase.initialize();
        Sys.println("source initialize");
    }
    function onStart(state) {
        Sys.println("source onStart");
        var saved = state;
        var missing = null;
        var isMissing = saved == missing;
        Sys.println(isMissing);
        if (isMissing) {
            Sys.println("null state");
        } else {
            Sys.println("dictionary state");
        }
        self.printAnswer();
        var again = true;
        while (again) {
            Sys.println("Boolean loop");
            again = false;
        }
        Sys.println(again == false);
        Sys.println(again);
    }
    function printAnswer() {
        Sys.println(Helpers.answer());
    }
}
