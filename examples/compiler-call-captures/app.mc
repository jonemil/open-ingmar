using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
module Trace {
    function mark(n as Lang.Number) as Lang.Number {
        System.println(n);
        return n;
    }
}
module Cold {
    var MARK as Lang.Number = Trace.mark(101);
    module Target {
        var MARK as Lang.Number = Trace.mark(102);
        module Deep {
            var MARK as Lang.Number = Trace.mark(103);
            function number(n as Lang.Number) as Lang.Number { return n; }
            function touch(n as Lang.Number) as Void { System.println(n); }
        }
    }
    module Peer {
        var MARK as Lang.Number = Trace.mark(104);
        function run() as Lang.Number {
            return me.Target.Deep.number(Trace.mark(105) > 0 ? Trace.mark(106) : 107);
        }
        function touch() as Void {
            self.Target.Deep.touch(Trace.mark(108) > 0 ? 109 : 110);
            self.Target.Deep.number(Trace.mark(111) > 0 ? 112 : 113);
        }
    }
}
module Direct {
    var MARK as Lang.Number = Trace.mark(201);
    function number(n as Lang.Number) as Lang.Number { return n; }
}
module Logic {
    var MARK as Lang.Number = Trace.mark(301);
    function value(flag as Lang.Boolean) as Lang.Number { return flag ? 1 : 0; }
}
class Token {
    var value as Lang.Number;
    function initialize(n as Lang.Number) {
        System.println(405);
        value = n;
    }
    function get() as Lang.Number { return value; }
}
module Objects {
    var MARK as Lang.Number = Trace.mark(401);
    function value(token as Token) as Lang.Number { return token.get(); }
}
module NestedOuter {
    var MARK as Lang.Number = Trace.mark(501);
    function value(n as Lang.Number) as Lang.Number { return n; }
}
module NestedInner {
    var MARK as Lang.Number = Trace.mark(502);
    function value(n as Lang.Number) as Lang.Number { return n; }
}
module Lex {
    var MARK as Lang.Number = Trace.mark(601);
    function number(n as Lang.Number) as Lang.Number { return n; }
    module Peer {
        var MARK as Lang.Number = Trace.mark(602);
        function run() as Lang.Number {
            return self.number(Trace.mark(603) > 0 ? 604 : 605);
        }
    }
}
function globalNumber(n as Lang.Number) as Lang.Number { return n; }
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        System.println(Cold.Peer.run());
        Cold.Peer.touch();
        System.println(Direct.number(Trace.mark(202) > 0 ? 203 : 204));
        System.println(Logic.value(Trace.mark(302) > 0 && Trace.mark(303) > 0));
        System.println(Objects.value(new Token(Trace.mark(402) > 0 ? Trace.mark(403) : 404)));
        System.println(NestedOuter.value(NestedInner.value(Trace.mark(503) > 0 ? 504 : 505)));
        System.println(Lex.Peer.run());
        System.println(globalNumber(Trace.mark(701) > 0 ? 702 : 703));
        System.exit();
    }
}
