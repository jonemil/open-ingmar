using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
function mark(n as Lang.Number) as Lang.Number { System.println(n); return n; }
module M {
    class Root {
        var field as Lang.Number = mark(10);
        function initialize(n as Lang.Number) { System.println(100); field = n; }
        protected function pick(n as Lang.Number) as Lang.Number { return field + n; }
        function virtual() as Lang.Number { return 1; }
        function forward() as Lang.Number { return virtual(); }
    }
}
class Leaf extends M.Root {
    var own as Lang.Number = mark(20);
    function initialize() { M.Root.initialize(mark(11)); field = 22; }
    function virtual() as Lang.Number { return 2; }
    protected function pick(n as Lang.Number) as Lang.Number { return field + n + 100; }
    private function secret() as Lang.Number { return own; }
    function run(flag as Lang.Boolean) {
        System.println(Leaf.pick(mark(3)));
        System.println(M.Root.pick(mark(4)));
        System.println(M.Root.forward());
        System.println(Leaf.secret());
        System.println(M.Root.pick(flag ? mark(5) : mark(6)));
        M.Root.initialize(mark(30));
        System.println(field);
        Leaf.initialize();
        System.println(field);
        System.println(own);
    }
}
class InitializerSelf {
    static var ready as Lang.Number = mark(300);
    function initialize() { System.println(400); }
    static function run() { InitializerSelf.initialize(); }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        var leaf = new Leaf();
        leaf.run(true);
        InitializerSelf.run();
        System.exit();
    }
}
