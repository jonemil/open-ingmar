using Toybox.Application;
using Toybox.System;

module WideOps {
    function longText(value as Toybox.Lang.Long or Null) as Toybox.Lang.String { return "l=" + value; }
    function floatText(value as Toybox.Lang.Float or Null) as Toybox.Lang.String { return value + "f"; }
    function doubleText(value as Toybox.Lang.Double or Null) as Toybox.Lang.String { return "d=" + value; }
    function left() as Toybox.Lang.String { System.println("left"); return "v="; }
    function right() as Toybox.Lang.Double { System.println("right"); return 2.5d; }
    function character() as Toybox.Lang.Char { System.println("char"); return 'A'; }
    function offset() as Toybox.Lang.Long { System.println("long"); return 1l; }
    function advance(c as Toybox.Lang.Char, n as Toybox.Lang.Long) as Toybox.Lang.Char { return c + n; }
    function reverse(n as Toybox.Lang.Long, c as Toybox.Lang.Char) as Toybox.Lang.Char { return n + c; }
    function strings() as Void {
        System.println("l=" + 4294967297l);
        System.println(-4294967297l + "l");
        System.println("f=" + 1.25);
        System.println(-1.25 + "f");
        System.println("d=" + 1.25d);
        System.println(-1.25d + "d");
        System.println("z=" + -0.0);
        System.println(-0.0d + "z");
        System.println("1" == 1l);
        System.println(1l != "1");
        System.println("1.25" == 1.25);
        System.println(1.25 != "1.25");
        System.println("1.25" == 1.25d);
        System.println(1.25d != "1.25");
        var l = "4294967297".toLong();
        var f = "1.25".toFloat();
        var d = "2.5".toDouble();
        System.println(WideOps.longText(l));
        System.println(WideOps.floatText(f));
        System.println(WideOps.doubleText(d));
        System.println("long" == l);
        System.println(f != "float");
        System.println("double" == d);
        l = null; f = null; d = null;
        System.println(WideOps.longText(l));
        System.println(WideOps.floatText(f));
        System.println(WideOps.doubleText(d));
        System.println(l == "long");
        System.println("float" != f);
        System.println(d == "double");
        var text = "v=";
        text += 4294967297l;
        text += 1.25;
        text += 2.5d;
        text += d;
        System.println(text);
        var texts = ["a="];
        texts[0] += 4294967297l;
        texts[0] += 1.25;
        texts[0] += WideOps.right();
        System.println(texts[0]);
        System.println(WideOps.left() + WideOps.right());
        System.println(WideOps.right() + WideOps.left());
        System.println("branch=" + (true ? 1.25d : 2.5d));
    }
    function chars() as Void {
        var characters = ['A'];
        characters[0] += WideOps.offset();
        System.println(characters[0]);
        System.println(WideOps.advance('A', 1l));
        System.println(WideOps.reverse(-1l, 'A'));
        System.println(WideOps.advance('A', 4294967297l));
        System.println('A' == 65l);
        System.println(65l == 'A');
        System.println('A' != 66l);
        System.println(66l != 'A');
        System.println('A' == 65.0);
        System.println(65.0 == 'A');
        System.println('A' != 66.0);
        System.println(66.0 != 'A');
        System.println('A' == 65.0d);
        System.println(65.0d == 'A');
        System.println('A' != 66.0d);
        System.println(66.0d != 'A');
        System.println('A'.weak().equals('A'));
        System.println('A'.weak().toString());
        var c = WideOps.character();
        c += WideOps.offset();
        System.println(c);
        System.println(WideOps.character() + WideOps.offset());
        System.println(WideOps.offset() + WideOps.character());
        var count = "2".toLong();
        if (count != null) { System.println(c + count); }
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { WideOps.strings(); WideOps.chars(); System.exit(); }
}
