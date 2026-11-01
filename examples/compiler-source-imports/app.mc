using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
using Library.Models as M;
using Library.Models as Build;
using M.Nested as N;
import Library.Models;
const FIRST=M.bump();
module Consumer {
    using Library.Models as Local;
    typedef Exact as Local.Token;
    function make() as Exact { return Local.make(7); }
}
function take(mode as Mode) as Amount { return mode as Lang.Number; }
class Holder {
    using Library.Models as Own;
    var token as Build.Token=new Build.Token(9);
    var M as Lang.Number=12;
    function get() as Own.Token { Own.bump(); return token; }
    function count() as Lang.Number { return M.COUNT; }
    function text() as Lang.String { return self.M.toString(); }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(FIRST);
        var token=new Token(41);
        System.println(token.get());
        System.println(token instanceof M.Token);
        System.println((token as M.Token).get());
        System.println(Models.bump());
        M.COUNT=10;
        M.COUNT++;
        System.println(M.COUNT);
        M.DATA[0]+=Models.bump();
        System.println(M.DATA[0]);
        System.println(N.answer());
        System.println(take(M.READY));
        System.println(M.Token.ON);
        System.println(Consumer.make().get());
        System.println(Consumer.extra().get());
        var holder=new Holder();
        System.println(holder.get().get());
        System.println(holder.text());
        System.println(holder.count());
        var M=[1,2,3];
        System.println(M.size());
        System.exit();
    }
}
