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

        function bare() as Inner {
            return new Inner(1);
        }

        function own() as Inner {
            return new self.Inner(2);
        }
    }

    class Sub extends Outer {}
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
        System.exit();
    }
}
