using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
class Root {
    function initialize() {}
    function pick(value as Lang.Number) as Lang.Number { return 11; }
    function inherited() as Lang.Number { return pick(0); }
}
class ProtectedLeaf extends Root {
    protected function pick(value as Lang.Number) as Lang.Number { return 22; }
    function own() as Lang.Number { return pick(0); }
    function through(value as ProtectedLeaf) as Lang.Number { return value.pick(0); }
}
class PrivateLeaf extends Root {
    private function pick(value as Lang.String) as Lang.String { return "private"; }
    function own() as Lang.String { return pick("x"); }
    function through(value as PrivateLeaf) as Lang.String { return value.pick("x"); }
}
class Grandchild extends PrivateLeaf {
    function pick(value as Lang.Number) as Lang.Number { return 33; }
}
function publicRoot(value as Root) as Lang.Number { return value.pick(0); }
function publicProtected(value as ProtectedLeaf) as Lang.Number { return value.pick(0); }
function publicPrivate(value as PrivateLeaf) as Lang.Number { return value.pick(0); }
typedef TextPicker as interface { function pick(value as Lang.String) as Lang.String; };
typedef NumberPicker as interface { function pick(value as Lang.Number) as Lang.Number; };
function textWitness(value as TextPicker) as Lang.String { return value.pick("x"); }
function numberWitness(value as NumberPicker) as Lang.Number { return value.pick(0); }
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        var p = new ProtectedLeaf();
        System.println(p.own());
        System.println(p.through(p));
        System.println(publicRoot(p));
        System.println(publicProtected(p));
        System.println(p.inherited());
        var q = new PrivateLeaf();
        System.println(q.own());
        System.println(q.through(q));
        System.println(publicRoot(q));
        System.println(publicPrivate(q));
        System.println(q.inherited());
        System.println(textWitness(q));
        System.println(numberWitness(p));
        var r = new Grandchild();
        System.println(publicRoot(r));
        System.println(publicPrivate(r));
        System.println(r.inherited());
        System.println(r.own());
        System.exit();
    }
}
