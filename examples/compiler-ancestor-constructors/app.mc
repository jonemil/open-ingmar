using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
using M as Types;
function mark(n as Lang.Number) as Lang.Number { System.println(n); return n; }
module M {
    class Root {
        protected var number as Lang.Number;
        private var text as Lang.String;
        var allocation as Lang.Number = mark(100);
        function initialize(n as Lang.Number) {
            number = n;
            text = "root";
            System.println(text);
            System.println(number);
        }
        function read() as Lang.Number { return number; }
    }
}
class Middle extends Types.Root {
    var items as Lang.Array<Lang.Number>;
    function initialize(n as Lang.Number) {
        Types.Root.initialize(n);
        items = [number, number + 1];
    }
}
class Leaf extends Middle {
    var next as Lang.Number;
    function initialize(n as Lang.Number) {
        Middle.initialize(mark(n));
        next = items[1] + 1;
        var ready = self;
        System.println(ready.read());
    }
    function read() as Lang.Number { return number + items[1] + next; }
    function reset() {
        $.M.Root.initialize(20);
        System.println(read());
    }
}
class VirtualRoot {
    var base as Lang.Number;
    function initialize() { base = 3; System.println(read()); }
    function read() as Lang.Number { return base; }
}
class VirtualLeaf extends VirtualRoot {
    var child as Lang.Number;
    function initialize() { child = 4; VirtualRoot.initialize(); }
    function read() as Lang.Number { return base + child; }
}
class PreparedMiddle extends M.Root {
    var combined as Lang.Number;
    function initialize() { combined = number + 1; }
}
class PreparedLeaf extends PreparedMiddle {
    function initialize() { M.Root.initialize(30); PreparedMiddle.initialize(); }
    function result() as Lang.Number { return combined; }
}
class BranchRoot {
    var value as Lang.Number;
    function initialize(first as Lang.Boolean) {
        if (first) { value = 11; } else { value = 22; }
        if (first) { return; }
        value = value + 11;
    }
}
class BranchChild extends BranchRoot {
    var total as Lang.Number;
    function initialize(first as Lang.Boolean) {
        BranchRoot.initialize(first);
        if (first) {
            total = value + 1;
            self.report();
            return;
        }
        total = value + 2;
        self.report();
    }
    function report() as Void { System.println(total); }
}
class SnapshotRoot {
    var snapshot as Lang.Number = read();
    function read() as Lang.Number { return 10; }
}
class SnapshotChild extends SnapshotRoot {
    var child as Lang.Number = 7;
    function read() as Lang.Number { return child; }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        var leaf = new Leaf(10);
        System.println(leaf.read());
        leaf.reset();
        var other = new Leaf(2);
        System.println(other.read());
        var virtualLeaf = new VirtualLeaf();
        System.println(virtualLeaf.read());
        var prepared = new PreparedLeaf();
        System.println(prepared.result());
        var first = new BranchChild(true);
        var second = new BranchChild(false);
        var snapshot = new SnapshotChild();
        System.println(snapshot.snapshot);
        System.exit();
    }
}
