using Toybox.Application;
using Toybox.System;

module Maps {
    function longs() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Long> { return {"wide" => 4294967297l, "min" => -9223372036854775808l, "max" => 9223372036854775807l}; }
    function floats() as Toybox.Lang.Dictionary<Toybox.Lang.Number, Toybox.Lang.Float> { return {1 => 1.25, 2 => -0.0}; }
    function doubles() as Toybox.Lang.Dictionary<Toybox.Lang.Long, Toybox.Lang.Double> { return {1l => 16777217.0d, 4294967297l => -0.0d, 1l => 4294967297.0d}; }
    function nullableLongs() as Toybox.Lang.Dictionary<Toybox.Lang.Long, Toybox.Lang.Long or Null> { return {1l => 4294967297l, 2l => null}; }
    function nullableFloats() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Float or Null> { return {"first" => 1.25, "null" => null}; }
    function nullableDoubles() as Toybox.Lang.Dictionary<Toybox.Lang.Boolean, Toybox.Lang.Double or Null> { return {true => 16777217.0d, false => null}; }
    function empty() as Toybox.Lang.Dictionary<Toybox.Lang.Long, Toybox.Lang.Double> { return {}; }
    function nullOnly() as Toybox.Lang.Dictionary<Toybox.Lang.Long, Toybox.Lang.Long or Null> { return {1l => null}; }
    function key(value as Toybox.Lang.Long) as Toybox.Lang.Long { System.println("key"); return value; }
    function value(value as Toybox.Lang.Double) as Toybox.Lang.Double { System.println("value"); return value; }
    function computed(flag as Toybox.Lang.Boolean) as Toybox.Lang.Dictionary<Toybox.Lang.Long, Toybox.Lang.Double> { return {Maps.key(1l) => Maps.value(1.25d), Maps.key(2l) => flag ? Maps.value(2.5d) : Maps.value(3.5d)}; }
    function showLong(n as Toybox.Lang.Long or Null) as Void { if (n == null) { System.println("null"); } else { System.println(n); } }
    function showFloat(n as Toybox.Lang.Float or Null) as Void { if (n == null) { System.println("null"); } else { System.println(n); } }
    function showDouble(n as Toybox.Lang.Double or Null) as Void { if (n == null) { System.println("null"); } else { System.println(n); } }
    function showNumber(n as Toybox.Lang.Number or Null) as Void { if (n == null) { System.println("null"); } else { System.println(n); } }
    function floatingKeys() as Void {
        var fs = {1.25 => 10, -0.0 => 20, 0.0 => 30, 1.25 => 40};
        System.println(fs.size()); Maps.showNumber(fs[1.25]); Maps.showNumber(fs[-0.0]); Maps.showNumber(fs[0.0]);
        fs.put(1.25, 50); System.println(fs.size()); Maps.showNumber(fs[1.25]);
        System.println(fs.keys().indexOf(1.25) >= 0);
        var ds = {1.25d => 10, -0.0d => 20, 0.0d => 30, 1.25d => 40};
        System.println(ds.size()); Maps.showNumber(ds[1.25d]); Maps.showNumber(ds[-0.0d]); Maps.showNumber(ds[0.0d]);
        ds.put(1.25d, 50); System.println(ds.size()); Maps.showNumber(ds[1.25d]);
        System.println(ds.keys().indexOf(1.25d) >= 0);
    }
    function run() as Void {
        var ls = Maps.longs();
        System.println(ls.size()); Maps.showLong(ls["wide"]); Maps.showLong(ls.get("min")); Maps.showLong(ls["max"]);
        Maps.showLong(ls["absent"]);
        var copy = ls; ls["wide"] = 7l; Maps.showLong(copy["wide"]); Maps.showLong(Maps.longs()["wide"]);
        ls.put("new", 8l); System.println(ls.hasKey("new")); ls.remove("new"); System.println(ls.hasKey("new"));
        System.println(ls.keys().indexOf("wide") >= 0); System.println(ls.values().indexOf(7l) >= 0);
        var fs = Maps.floats(); Maps.showFloat(fs[1]); Maps.showFloat(fs[2]);
        fs[1] = 2.5; fs.put(3, 7.5); System.println(fs.size()); System.println(fs.values().indexOf(7.5) >= 0);
        Maps.showFloat(Maps.floats()[1]); fs.remove(3); System.println(fs.hasKey(3));
        var ds = Maps.doubles(); System.println(ds.size()); Maps.showDouble(ds[1l]); Maps.showDouble(ds[4294967297l]);
        ds[Maps.key(1l)] = Maps.value(2.5d); Maps.showDouble(ds[1l]);
        ds.put(3l, 7.5d); System.println(ds.hasKey(3l)); ds.remove(3l); System.println(ds.hasKey(3l));
        System.println(ds.keys().indexOf(4294967297l) >= 0); System.println(ds.values().indexOf(2.5d) >= 0);
        var nl = Maps.nullableLongs(); var nf = Maps.nullableFloats(); var nd = Maps.nullableDoubles();
        Maps.showLong(nl[1l]); Maps.showLong(nl[2l]); Maps.showFloat(nf["first"]); Maps.showFloat(nf["null"]); Maps.showDouble(nd[true]); Maps.showDouble(nd[false]);
        System.println(nl.hasKey(2l)); System.println(nf.hasKey("null")); System.println(nd.hasKey(false));
        var saved = nd[true]; nd[true] = null; Maps.showDouble(saved); Maps.showDouble(nd[true]);
        nl[1l] = null; nf.put("first", null); Maps.showLong(nl[1l]); Maps.showFloat(nf["first"]);
        System.println(nl.values().indexOf(null) >= 0); System.println(nf.values().indexOf(null) >= 0); System.println(nd.values().indexOf(null) >= 0);
        Maps.showLong(Maps.nullOnly()[1l]);
        var empty = Maps.empty(); System.println(empty.isEmpty()); empty.put(9l, 1.25d); Maps.showDouble(empty[9l]);
        var made = Maps.computed(true); Maps.showDouble(made[1l]); Maps.showDouble(made[2l]);
        made = Maps.computed(false); Maps.showDouble(made[2l]);
        var text = Maps.longs().toString(); System.println(text.find("4294967297") != null);
        System.println(ds.equals(ds)); System.println(ds.equals(Maps.doubles()));
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Maps.run(); Maps.floatingKeys(); System.exit(); }
}
