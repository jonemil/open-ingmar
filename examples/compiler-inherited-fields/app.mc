using Toybox.Application;
using Toybox.Lang;
using Toybox.System;

function mark(n as Lang.Number) as Lang.Number {
    System.println(n);
    return n;
}
class Root {
    var root as Lang.Number = mark(1);
    protected var step as Lang.Number = mark(2);
    private var secret as Lang.Number = mark(3);
    const tag = "root";
    var items as Lang.Array<Lang.Number> = [1];
    var literal as Lang.Number = 7;
    function initialize() {}
    function readSecret() as Lang.Number { return secret; }
}
class Middle extends Root {
    var middle as Lang.Number = mark(root + 3);
    function initialize() {}
    function total() as Lang.Number { return root + step + middle; }
}
class Leaf extends Middle {
    private var secret as Lang.String = "leaf";
    var leaf as Lang.Number = mark(5);
    function initialize() {}
    function update() as Lang.Number {
        root += 10;
        self.step += 20;
        return root + step + middle + leaf;
    }
    function readLeaf() as Lang.String { return secret; }
}
function read(value as Root) as Lang.Number { return value.root; }
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        var value = new Leaf();
        System.println(value.total());
        System.println(value.readSecret());
        System.println(value.readLeaf());
        System.println(value.tag);
        System.println(value.literal);
        System.println(value.update());
        System.println(read(value));
        value.root = 40;
        System.println(value.total());
        System.println(value.readSecret());
        var other = new Leaf();
        System.println(other.root);
        System.println(value.root);
        value.items.add(9);
        System.println(other.items.size());
        System.println(value.items.size());
        System.exit();
    }
}
