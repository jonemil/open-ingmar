class Counter extends Application.AppBase {
    function start() {
        System.println("class startup");
        System.println(self.answer(self.left(), self.right()));
        self.announce();
        self.count(3);
        System.println(Numbers.answer());
    }
    function answer(left, right) {
        return left - right;
    }
    function left() {
        System.println(1);
        return 50;
    }
    function right() {
        System.println(2);
        return 3;
    }
    function announce() {
        System.println("receiver call");
    }
    function count(limit) {
        var n = limit;
        while (n > 0) {
            if (n == 1) {
                return;
            }
            System.println(self.answer(n, 0));
            n = n - 1;
        }
    }
}
