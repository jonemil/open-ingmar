using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
using Models as Types;

// Parents intentionally follow their consumers.
module Models {
    class Leaf extends Middle {
        var value as Lang.Number;
        function initialize(n as Lang.Number) { value = n; }
    }
    class Middle extends Root {
        function initialize() {}
    }
    class Root {
        function initialize() {}
    }
}
function identity(value as Types.Root) as Types.Root { return value; }
function count(values as Lang.Array<Types.Root>) as Lang.Number { return values.size(); }
function guarded(value as Types.Root) as Lang.Number {
    if (value instanceof Types.Leaf) { return value.value; }
    return 0;
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        var leaf = new Types.Leaf(41);
        System.println(leaf instanceof Types.Root);
        System.println(leaf instanceof Types.Middle);
        System.println(leaf instanceof Types.Leaf);
        System.println(leaf instanceof Lang.Object);
        var root = identity(leaf);
        System.println(root instanceof $.Models.Leaf);
        System.println(guarded(root));
        System.println(guarded(new Types.Root()));
        System.println(count([leaf]));
        System.exit();
    }
}
