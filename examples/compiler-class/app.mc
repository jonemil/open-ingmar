using Toybox.Application as App;

class Counter extends App.AppBase {
    using Toybox.System as Sys;
    function initialize() {
        App.AppBase.initialize();
        Sys.println("source initialize");
    }
    function start() {
        Sys.println("class startup");
        Sys.println(self.answer(self.left(), self.right()));
        self.announce();
        self.count(3);
        Sys.println(Numbers.answer());
    }
    function answer(left, right) {
        return left - right;
    }
    function left() {
        Sys.println(1);
        return 50;
    }
    function right() {
        Sys.println(2);
        return 3;
    }
    function announce() {
        Sys.println("receiver call");
    }
    function count(limit) {
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
