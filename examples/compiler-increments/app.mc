using Toybox.Application;
using Toybox.System;
module Updates {
    function index() { System.println("index"); return 0; }
    function array(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Array<Toybox.Lang.Number> { System.println("array"); return a; }
}
class MinimalConsoleApp extends Application.AppBase {
    var count = 10;
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        var n = 1;
        n++; System.println(n);
        ++n; System.println(n);
        n--; System.println(n);
        --n; System.println(n);
        self.count++; System.println(self.count);
        ++self.count; System.println(self.count);
        self.count--; System.println(self.count);
        --self.count; System.println(self.count);
        var a = [10, 20];
        a[Updates.index()]++; System.println(a[0]);
        ++a[Updates.index()]; System.println(a[0]);
        Updates.array(a)[Updates.index()]--; System.println(a[0]);
        --Updates.array(a)[Updates.index()]; System.println(a[0]);
        n = 2147483647; n++; System.println(n);
        n = -2147483648; --n; System.println(n);
        var found = "7".toNumber();
        if (found != null) { found++; }
        if (found != null) { System.println(found); }
        var limit = 0;
        while (limit < 2) { limit++; System.println(limit); }
        System.exit();
    }
}
