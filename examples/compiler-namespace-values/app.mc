using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
function identity(value) { return value; }
function globalOwner() { return me; }
function globalNumber(n as Lang.Number) as Lang.Number { return n + 1; }
module Trace {
    var LOG as Lang.Number = 0;
    function mark(n as Lang.Number) as Lang.Number { LOG = LOG * 1000 + n; return n; }
}
module Cold {
    var MARK as Lang.Number = Trace.mark(101);
    module Child {
        var MARK as Lang.Number = Trace.mark(102);
        function owner() { return self; }
        function number(n as Lang.Number) as Lang.Number { return n + 10; }
    }
}
class Token { function initialize() {} function plain() as Lang.Number { return 23; } }
var STORED_MODULE = Cold.Child;
var STORED_CLASS = Token;
class Holder {
    var heldModule = Cold.Child;
    var heldClass = Token;
    function getModule() { return heldModule; }
    function getClass() { return heldClass; }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        System.println(Trace.LOG);
        var moduleValue = Cold.Child;
        var returned = identity(moduleValue);
        System.println(returned == Cold.Child);
        System.println(Cold.Child.owner() == moduleValue);
        var classValue = Token;
        System.println(identity(classValue) == Token);
        var globals = $;
        System.println(globalOwner() == globals);
        var array = [moduleValue, classValue, globals];
        System.println(array[0] == Cold.Child);
        System.println(array[1] == Token);
        System.println(array[2] == $);
        System.println(Cold.Child != Cold);
        System.println(Token != Cold.Child);
        var holder = new Holder();
        System.println(holder.getModule() == STORED_MODULE);
        System.println(holder.getClass() == STORED_CLASS);
        var core = Lang;
        System.println(core == $.Toybox.Lang);
        var numberClass = Lang.Number;
        System.println(numberClass == $.Toybox.Lang.Number);
        var viewClass = Toybox.WatchUi.View;
        System.println(viewClass == $.Toybox.WatchUi.View);
        System.println(moduleValue has :number);
        System.println(moduleValue has :method);
        System.println(Trace.LOG);
        System.exit();
    }
}
