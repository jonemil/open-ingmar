using Toybox.Application;
using Toybox.System;

module Real {
    function pass(value as Toybox.Lang.Float) as Toybox.Lang.Float { return value; }
    function effect(value as Toybox.Lang.Float) as Toybox.Lang.Float {
        System.println("float"); return value;
    }
    function number(value as Toybox.Lang.Number) as Toybox.Lang.Number {
        System.println("number"); return value;
    }
    function parse(value as Toybox.Lang.String) as Toybox.Lang.Float or Null { return value.toFloat(); }
    function show(value as Toybox.Lang.Float or Null) as Void {
        if (value == null) { System.println("null"); } else { System.println(value.format("%.3f")); }
    }
    function choose(flag as Toybox.Lang.Boolean) as Toybox.Lang.Float or Null { return flag ? 1.25 : null; }
    function doublePass(value as Toybox.Lang.Double) as Toybox.Lang.Double { return value; }
    function doubleParse(value as Toybox.Lang.String) as Toybox.Lang.Double or Null { return value.toDouble(); }
    function showDouble(value as Toybox.Lang.Double or Null) as Void {
        if (value == null) { System.println("null"); } else { System.println(value.format("%.6f")); }
    }
    function doubles() as Void {
        var x = Real.doublePass(-5.75d);
        System.println(x);
        System.println(Real.doublePass(0.0d));
        System.println(Real.doublePass(-0.0d));
        System.println(Real.doublePass(1.25e2d));
        System.println(Real.doublePass(16777217.0d).format("%.0f"));
        System.println(x.abs());
        System.println(x.toDouble());
        System.println(x.toFloat());
        System.println(x.toNumber());
        System.println(x.toLong());
        System.println(x.toString());
        System.println(1.toDouble());
        System.println(4294967297l.toDouble().format("%.0f"));
        System.println(Real.pass(1.25).toDouble());
        System.println(x.equals(-5.75d));
        System.println(1.0d.equals(1.0));
        System.println(1.0.equals(1.0d));
        System.println(x.hashCode() == x.hashCode());
        System.println(x.weak().equals(x));
        System.println(x.compareTo(-5.75) == 0);
        System.println(1.0.compareTo(1.0d) == 0);
        System.println(1.compareTo(1.0d) == 0);
        System.println(x + 2);
        System.println(2 + x);
        System.println(x + 2l);
        System.println(2l + x);
        System.println(x + 2.0);
        System.println(2.0 + x);
        System.println(x - 2.0d);
        System.println(x * 2.0d);
        System.println(x / 2.0d);
        System.println(-x);
        System.println(+x);
        System.println(-Real.doublePass(0.0d));
        System.println(x < 0);
        System.println(x != 0.0);
        System.println(x <= -5.75d);
        System.println(x >= -5.75d);
        x += 1l;
        x -= 0.25;
        x *= 2;
        x /= 2.0d;
        x++;
        --x;
        System.println(x);
        System.println(true ? 1.25d : 2.5d);
        Real.showDouble(Real.doubleParse("bad"));
        Real.showDouble(Real.doubleParse("1.234567"));
        Real.showDouble(Real.doubleParse("-3.5tail"));
        Real.effect(-1.0).toDouble();
    }
    function mixed(left as Toybox.Lang.Long, right as Toybox.Lang.Float) as Toybox.Lang.Double { return left + right; }
    function run() as Void {
        System.println(Real.pass(1.25));
        System.println(Real.pass(0.0));
        System.println(Real.pass(-0.0));
        System.println(Real.pass(1.25e2));
        System.println(Real.pass(1e2));
        System.println(Real.doublePass(1d));
        System.println(Real.pass(1.25e-2).format("%.4f"));
        System.println(Real.pass(16777217.0).format("%.0f"));
        var x = Real.pass(-5.75);
        System.println(x.abs().format("%.2f"));
        System.println(x.toFloat().format("%.2f"));
        System.println(x.toNumber());
        System.println(x.toLong());
        System.println(x.toString());
        System.println(Real.number(-7).toFloat().format("%.1f"));
        System.println(4294967297l.toFloat().format("%.0f"));
        System.println(x.hashCode() == Real.pass(-5.75).hashCode());
        System.println(x.weak().equals(x));
        System.println(x.equals(-5.75));
        System.println(Real.pass(1.0).equals(1));
        System.println(1.equals(1.0));
        System.println(1l.equals(1.0));
        System.println("1".equals(1.0));
        System.println(false.equals(0.0));
        System.println('A'.equals(65.0));
        System.println({}.equals(1.0));
        System.println(x.equals(null));
        System.println(Real.pass(3.0).compareTo(3) == 0);
        System.println(Real.pass(3.0).compareTo(3l) == 0);
        System.println(Real.pass(1.0).compareTo(true) == 0);
        System.println(Real.pass(65.0).compareTo('A') == 0);
        System.println(3.compareTo(3.0) == 0);
        System.println(3l.compareTo(3.0) == 0);
        System.println(true.compareTo(1.0) == 0);
        System.println('A'.compareTo(65.0) == 0);
        System.println(Real.effect(2.5) + Real.number(3));
        System.println(3 + Real.pass(2.5));
        System.println(Real.mixed(3l, Real.pass(2.5)));
        System.println(Real.pass(2.5) + 3l);
        System.println(Real.pass(2.5) - 3);
        System.println(3 - Real.pass(2.5));
        System.println(Real.pass(2.5) * 3);
        System.println(Real.pass(5.0) / 2);
        System.println(+x);
        System.println(-x);
        System.println(-Real.pass(0.0));
        System.println(Real.pass(2.5) < 3);
        System.println(3l > Real.pass(2.5));
        System.println(Real.pass(3.0) == 3);
        System.println(Real.pass(3.0) != 4l);
        System.println(Real.pass(3.0) <= 3.0);
        System.println(Real.pass(3.0) >= 3.0);
        x += 1;
        x *= 2;
        x /= 2.0;
        x -= 0.25;
        System.println(x);
        x++;
        System.println(x);
        --x;
        System.println(x);
        System.println(true ? 1.25 : 2.5);
        var parsed = Real.choose(false);
        while (parsed == null) { parsed = Real.parse("42.125tail"); }
        System.println(parsed.abs());
        var copy = parsed;
        parsed = null;
        Real.show(copy);
        Real.show(Real.parse("bad"));
        Real.show(Real.parse("1.25e2"));
        Real.show(Real.parse("-3.5"));
        Real.show(Real.parse("0"));
        Real.show(Real.choose(true));
        Real.effect(-3.0).abs();
        Real.effect(1.0).toFloat();
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Real.run(); Real.doubles(); System.exit(); }
}
