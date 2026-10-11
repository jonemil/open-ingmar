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
        if (saved == missing) {
            Sys.println("null state");
        } else {
            Sys.println("dictionary state");
        }
        self.printAnswer();
    }
    function printAnswer() {
        Sys.println(Helpers.answer());
    }
}
