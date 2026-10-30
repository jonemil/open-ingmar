using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
module Trace {
    function number(value as Lang.Number) as Lang.Number {
        System.println(value);
        return value;
    }
}
var GLOBAL=Trace.number(10), EMPTY;
var TYPED as Lang.Number or Null;
var TEXT as Lang.String="global";
var DATA as Lang.Array<Lang.Number> = [1,2];
module M {
    var VALUE as Lang.Number=Trace.number(20), LABEL="module";
    function change() { self.VALUE += 2; LABEL="changed"; }
}
module Late { var VALUE as Lang.Number=Trace.number(50); }
function local() { var GLOBAL=5; GLOBAL++; return GLOBAL; }
function parameter(GLOBAL as Lang.Number) { GLOBAL+=1; return GLOBAL; }
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println("begin");
        System.println(GLOBAL);
        System.println(EMPTY==null);
        System.println(TYPED==null);
        var saved=GLOBAL;
        GLOBAL="new";
        EMPTY=9;
        System.println(GLOBAL.length());
        System.println(saved==10);
        System.println(EMPTY);
        M.VALUE=Trace.number(30);
        System.println(M.VALUE);
        M.VALUE+=Trace.number(40);
        M.VALUE++;
        M.change();
        System.println(M.VALUE);
        System.println(M.LABEL.length());
        TEXT=true ? "changed" : "unused";
        System.println(TEXT.length());
        DATA=[3,4,5];
        DATA[0]+=1;
        System.println(DATA[0]);
        System.println(DATA.size());
        System.println(local());
        System.println(parameter(8));
        System.println(GLOBAL.length());
        M.another();
        System.println(M.EXTRA);
        System.println(M.VALUE);
        Late.VALUE=true ? Trace.number(60) : Trace.number(70);
        System.println(Late.VALUE);
        System.exit();
    }
}
