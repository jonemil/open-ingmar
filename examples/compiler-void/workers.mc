module Startup {
    function begin() {
        System.println("start");
    }
    function show(left, right) {
        System.println(left - right);
        return;
    }
    function count(limit) {
        var n = limit;
        while (n > 0) {
            if (n == 1) {
                return;
            }
            System.println(n);
            n = n - 1;
        }
        System.println("zero");
    }
}
module Numbers {
    function left() {
        System.println(1);
        return 50;
    }
    function right() {
        System.println(2);
        return 3;
    }
    function answer() {
        System.println(99);
        return 7;
    }
}
