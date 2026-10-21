using Toybox.Application as App;

class Counter extends App.AppBase {
    using Toybox.System as Sys;
    function initialize() {
        App.AppBase.initialize();
        Sys.println("source initialize");
    }
    function start() as Void {
        Sys.println("class startup");
        Sys.println(self.answer(self.left(), self.right()));
        self.announce();
        self.count(3);
        Sys.println(Numbers.answer());
    }
    function answer(left as Toybox.Lang.Number, right as Toybox.Lang.Number) as Toybox.Lang.Number {
        return left - right;
    }
    function left() as Toybox.Lang.Number {
        Sys.println(1);
        return 50;
    }
    function right() as Toybox.Lang.Number {
        Sys.println(2);
        return 3;
    }
    function announce() as Void {
        Sys.println("receiver call");
    }
    function count(limit as Toybox.Lang.Number) as Void {
        var n = limit;
        while (n > 0) {
            if (n == 1) {
                return;
            }
            Sys.println(self.answer(n, 0));
            n = n - 1;
        }
    }
}
