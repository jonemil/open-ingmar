using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
const ROOT=5;
function globalValue() as Lang.Number { return 3; }
module Trace {
    function mark(value as Lang.Number) as Lang.Number {
        System.println(value);
        return value;
    }
}
module Tree {
    typedef Num as Lang.Number;
    var STATE as Num=Trace.mark(10);
    const BASE=7;
    function parentValue() as Num { return STATE; }
    module Branch {
        var MARK as Num=Trace.mark(20);
        enum Kind { ONE=1, TWO=2 }
        function value() as Num { return MARK+BASE; }
        module Leaf {
            var MARK as Num=Trace.mark(30);
            function root() as Num { return ROOT; }
            function value() as Num { return MARK+BASE; }
            function parent() as Num { return parentValue(); }
            function global() as Num { return globalValue(); }
            function kind(value as Kind) as Kind { return value; }
        }
    }
}
module Writes {
    var VALUE as Lang.Number=Trace.mark(40);
    module Inner {
        var MARK as Lang.Number=Trace.mark(50);
        module Leaf {
            var MARK as Lang.Number=Trace.mark(60);
            function store() { VALUE=Trace.mark(99); }
        }
    }
}
module Direct {
    var VALUE as Lang.Number=Trace.mark(70);
    module Inner {
        var MARK as Lang.Number=Trace.mark(80);
        module Leaf {
            function store() { Direct.VALUE=Trace.mark(77); }
        }
    }
}
module Empty { module Child {} }
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println("begin");
        System.println(Tree.Branch.Leaf.root());
        System.println(Tree.Branch.Leaf.value());
        System.println(Tree.Branch.extra());
        System.println(Tree.Branch.Leaf.parent());
        System.println(Tree.Branch.Leaf.global());
        System.println($.Tree.Branch.Leaf.kind(Tree.Branch.ONE));
        Writes.Inner.Leaf.store();
        System.println(Writes.VALUE);
        Direct.Inner.Leaf.store();
        System.println(Direct.VALUE);
        System.println(Direct.Inner.MARK);
        System.exit();
    }
}
