using Toybox.Application;
using Toybox.Lang;
using Toybox.System;

class Leaf extends Middle {
    var offset as Lang.Number = 100;
    function initialize() {}
    // Undecorated overrides inherit the existing Number contracts.
    function value(n) { return n + offset; }
    protected function step(n) { return n + offset; }
    // Parent-private methods have separate selectors, not override contracts.
    private function secret() as Lang.String { return "leaf"; }
}
class Middle extends Root {
    function initialize() {}
    function twice(n as Lang.Number) as Lang.Number { return inherited(n) + inherited(n); }
}
class Root {
    function initialize() {}
    function value(n as Lang.Number) as Lang.Number { return n + 1; }
    protected function step(n as Lang.Number) as Lang.Number { return n + 1; }
    private function secret() as Lang.Number { return 5; }
    function inherited(n as Lang.Number) as Lang.Number { return step(n) + secret(); }
    function privateValue() as Lang.Number { return secret(); }
}
function call(value as Root, n as Lang.Number) as Lang.Number { return value.value(n); }
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        var leaf = new Leaf();
        System.println(leaf.inherited(1));
        System.println(leaf.value(2));
        System.println(call(leaf, 3));
        System.println(leaf.twice(1));
        System.println(leaf.privateValue());
        var root = new Root();
        System.println(root.inherited(1));
        System.println(call(root, 3));
        var bound = leaf.method(:value);
        System.println(bound.invoke(4));
        var inheritedBound = leaf.method(:privateValue);
        System.println(inheritedBound.invoke());
        System.println(leaf has :secret);
        System.exit();
    }
}
