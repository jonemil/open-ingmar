using Toybox.System;
using Toybox.WatchUi;

class ReplacementView extends WatchUi.View {
    function initialize() { View.initialize(); }
}

module Writes {
    function chars(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Toybox.Lang.Array<Toybox.Lang.Char> {
        System.println("char receiver");
        return a;
    }
    function views(a as Toybox.Lang.Array<Toybox.WatchUi.View>) as Toybox.Lang.Array<Toybox.WatchUi.View> {
        System.println("view receiver");
        return a;
    }
    function index() as Toybox.Lang.Number { System.println("index"); return 1; }
    function character() as Toybox.Lang.Char {
        System.println("char value");
        return 'Ω';
    }
    function flag() as Toybox.Lang.Boolean { System.println("flag"); return true; }
    function choose(flag as Toybox.Lang.Boolean) as Toybox.WatchUi.View {
        System.println("view value");
        if (flag) { return new WatchUi.View(); }
        return new WatchUi.View();
    }
    function run() as Void {
        var chars = ['a', 'b'];
        var sharedChars = chars;
        Writes.chars(chars)[Writes.index()] = Writes.character();
        System.println(sharedChars[1]);
        System.println(chars.size());
        chars[0] = chars[1].toLower();
        System.println(sharedChars[0]);
        chars[0] = (false || true).toString().toCharArray()[0];
        System.println(sharedChars[0]);

        var original = new WatchUi.View();
        var views = [original, original];
        var sharedViews = views;
        Writes.views(views)[Writes.index()] = Writes.choose(false || Writes.flag());
        System.println(sharedViews.indexOf(original));
        System.println(sharedViews.indexOf(views[1]));
        System.println(sharedViews.size());
        var replacement = new ReplacementView();
        views[0] = replacement;
        System.println(sharedViews.indexOf(original));
        System.println(sharedViews.indexOf(replacement));
        views[1] = views[0];
        System.println(sharedViews.indexOf(replacement));
        System.println(sharedViews.size());
        System.println("done");
    }
}
