using Toybox.Application;
using Toybox.Lang;
using Toybox.System;

module Scope {
    var number as Lang.Number = 40;

    class Outer {
        var number as Lang.Number = 90;

        class Inner {
            var value as Lang.Number;

            function initialize(n as Lang.Number) {
                value = n;
            }

            function get() as Lang.Number {
                return number + value;
            }
        }

        class Middle {
            private class Leaf {
                function get() as Lang.Number { return 51; }
            }
        }

        function bare() as Inner {
            return new Inner(1);
        }

        function own() as Inner {
            return new self.Inner(2);
        }
    }

    class Sub extends Outer {
        function inherited() as Outer.Inner {
            return new Inner(6);
        }
    }
}

class Marker {
    function get() as Lang.Number { return 99; }
}

class RootOuter {
    class Marker {
        function get() as Lang.Number { return 61; }
    }
}

class ChildOuter extends RootOuter {
    class Marker {
        function get() as Lang.Number { return 62; }
    }

    function make() as Marker {
        return new Marker();
    }
}

class MinimalConsoleApp extends Application.AppBase {
    function initialize() {
        AppBase.initialize();
    }

    function onStart(state) {
        var outer = new Scope.Outer();
        var sub = new Scope.Sub();
        System.println(outer.bare().get());
        System.println(outer.own().get());
        System.println((new outer.Inner(3)).get());
        System.println((new sub.Inner(4)).get());
        System.println((new Scope.Outer.Inner(5)).get());
        System.println(sub.inherited().get());
        System.println((new outer.Middle.Leaf()).get());
        System.println((new Scope.Outer.Middle.Leaf()).get());
        System.println((new RootOuter.Marker()).get());
        var child = new ChildOuter();
        System.println((new child.Marker()).get());
        System.println(child.make().get());
        System.exit();
    }
}
