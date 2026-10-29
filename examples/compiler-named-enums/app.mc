using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
enum Color { RED, GREEN=5, BLUE }
enum Other { OTHER }
const SELECTED as Chosen = BLUE;
module M {
    enum Kind { FIRST=3, SECOND, COPY=BLUE, AFTER_COPY }
}
module Fns {
    function echo(value as Color) as Color { return value; }
    function number(value as Color) as Lang.Number { return value; }
    function make() as Lang.Array<Color> { return [BLUE, RED]; }
    function table() as Lang.Dictionary<Color, Color> { return {RED => GREEN} as Lang.Dictionary<Color, Color>; }
    function maybe(value as Color or Null) as Color or Null { return value; }
}
class Box {
    var value as Color;
    function initialize() { self.value=RED; }
    function read() as Color { return self.value; }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(Fns.echo(SELECTED));
        System.println(Fns.number(BLUE));
        System.println(BLUE.toString());
        System.println(BLUE+1);
        System.println(-BLUE);
        System.println(GREEN | BLUE);
        System.println(RED == OTHER);
        System.println(M.SECOND);
        System.println(M.COPY);
        System.println(M.AFTER_COPY);
        System.println(M.get());
        var values=Fns.make();
        values.add(GREEN);
        values.sort(null);
        System.println(Fns.echo(values[0]));
        System.println(Fns.echo(values[2]));
        var table=Fns.table();
        var found=table[RED];
        if (found != null) { System.println(Fns.echo(found)); }
        table[BLUE]=RED;
        System.println(table.size());
        var optional=Fns.maybe(null);
        System.println(optional == null);
        var box=new Box();
        box.value=BLUE;
        System.println(Fns.echo(box.read()));
        System.println(Fns.echo(1 as Color));
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
