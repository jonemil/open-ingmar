using Toybox.Application;
using Toybox.System;

module Numbers {
    function longs() as Toybox.Lang.Array<Toybox.Lang.Long> { return [0l, 4294967297l, -9223372036854775808l, 9223372036854775807l]; }
    function floats() as Toybox.Lang.Array<Toybox.Lang.Float> { return [0.0, -0.0, 1.25, -5.75]; }
    function doubles() as Toybox.Lang.Array<Toybox.Lang.Double> { return [0.0d, -0.0d, 16777217.0d, -5.75d]; }
    function nullableLongs() as Toybox.Lang.Array<Toybox.Lang.Long or Null> { return [4294967297l, null]; }
    function nullableFloats() as Toybox.Lang.Array<Toybox.Lang.Float or Null> { return [1.25, null]; }
    function nullableDoubles() as Toybox.Lang.Array<Toybox.Lang.Double or Null> { return [16777217.0d, null]; }
    function emptyLongs() as Toybox.Lang.Array<Toybox.Lang.Long> { return []; }
    function emptyFloats() as Toybox.Lang.Array<Toybox.Lang.Float> { return []; }
    function emptyDoubles() as Toybox.Lang.Array<Toybox.Lang.Double> { return []; }
    function effect(value as Toybox.Lang.Double) as Toybox.Lang.Double { System.println("double"); return value; }
    function computed(flag as Toybox.Lang.Boolean) as Toybox.Lang.Array<Toybox.Lang.Double> { return [Numbers.effect(1.25d), flag ? Numbers.effect(2.5d) : Numbers.effect(3.5d)]; }
    function run() as Void {
        var ls = Numbers.longs();
        System.println(ls.size()); System.println(ls[1]); System.println(ls[2]); System.println(ls[3]);
        ls[0] = 4294967297l; ls[0] += 1; ls[0]++; System.println(ls[0]);
        System.println(Numbers.longs()[0]);
        System.println(ls.indexOf(4294967297l));
        System.println(ls.add(9l).size());
        var lc = ls.slice(0, 2); ls[1] = 7l; System.println(lc[1]);
        System.println(ls.remove(9l));
        System.println(ls.addAll([8l, 9l]).size());
        System.println(ls.removeAll(8l));
        var fs = Numbers.floats();
        System.println(fs[0]); System.println(fs[1]); System.println(fs[2]);
        fs[0] = 2.5; fs[0] *= 2; fs[0]++; System.println(fs[0]);
        System.println(Numbers.floats()[0]);
        System.println(fs.indexOf(1.25));
        System.println(fs.add(9.5).size());
        var fc = fs.slice(0, 3); fs[2] = 7.0; System.println(fc[2]);
        System.println(fs.remove(9.5));
        System.println(fs.addAll([8.5, 9.5]).size());
        System.println(fs.removeAll(8.5));
        var ds = Numbers.doubles();
        System.println(ds[0]); System.println(ds[1]); System.println(ds[2]);
        ds[0] = 4294967297.0d; ds[0] += 1l; --ds[0]; System.println(ds[0].format("%.0f"));
        System.println(Numbers.doubles()[0]);
        System.println(ds.indexOf(16777217.0d));
        System.println(ds.add(9.5d).size());
        var dc = ds.slice(0, 3); ds[2] = 7.0d; System.println(dc[2]);
        System.println(ds.remove(9.5d));
        System.println(ds.addAll([8.5d, 9.5d]).size());
        System.println(ds.removeAll(8.5d));
        var nl = Numbers.nullableLongs(); var nf = Numbers.nullableFloats(); var nd = Numbers.nullableDoubles();
        var l = nl[0]; if (l != null) { System.println(l); }
        var f = nf[0]; if (f != null) { System.println(f); }
        var d = nd[0]; if (d != null) { System.println(d); }
        System.println(nl.indexOf(null)); System.println(nf.indexOf(null)); System.println(nd.indexOf(null));
        nl[0] = null; nf[0] = null; nd[0] = null;
        System.println(nl[0] == null); System.println(nf[0] == null); System.println(nd[0] == null);
        nl.add(7l); nf.add(7.5); nd.add(7.5d);
        System.println(nl.remove(null)); System.println(nf.remove(null)); System.println(nd.remove(null));
        var made = Numbers.computed(true); System.println(made[0]); System.println(made[1]);
        made = Numbers.computed(false); System.println(made[1]);
        System.println(Numbers.emptyLongs().add(1l).size());
        System.println(Numbers.emptyFloats().add(1.0).size());
        System.println(Numbers.emptyDoubles().add(1.0d).size());
        var lr = ls.reverse(); var fr = fs.reverse(); var dr = ds.reverse();
        System.println(lr[0]); System.println(fr[0]); System.println(dr[0]);
        System.println(ls.toString()); System.println(fs.toString()); System.println(ds.toString());
        System.println(nl.addAll([8l, null]).slice(0, 4).size());
        System.println(nf.addAll([8.5, null]).reverse().size());
        System.println(nd.addAll([8.5d, null]).slice(null, null).size());
        System.println(ls.size()); System.println(fs.size()); System.println(ds.size());
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Numbers.run(); System.exit(); }
}
