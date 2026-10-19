using Toybox.Lang;
using Toybox.System;
using Toybox.Graphics;
class Token {
    var value = 0;
    function initialize(n) { self.value = n; }
    function get() { return self.value; }
}
module Objects {
    function constants() as Lang.Array<Lang.Object> { return [1, 2l, 3.5, 4.5d, true, 'A', "text"]; }
    function optional() as Lang.Array<Lang.Object or Null> { return [null, 1, "text"]; }
    function empty() as Lang.Array<Lang.Object> { return []; }
    function sized() as Lang.Array<Lang.Object or Null> { return new[2]; }
    function enums() as Lang.Array<Lang.Object> { return [Lang.NUMBER_FORMAT_UINT8, Lang.ENDIAN_BIG, Graphics.FONT_MEDIUM]; }
    function effect(value as Lang.Object) as Lang.Object { System.println(value.toString()); return value; }
    function computed(flag as Lang.Boolean) as Lang.Array<Lang.Object> {
        return [Objects.effect(1), flag ? Objects.effect("yes") : Objects.effect(false)];
    }
    function references(t as Token, b as Lang.ByteArray, a as Lang.Array<Lang.Number>, d as Lang.Dictionary<Lang.String,Lang.Number>, m as Lang.Method, w as Lang.WeakReference) as Lang.Array<Lang.Object> {
        return [t, b, a, d, m, w, :name];
    }
    function mapping(t as Token, b as Lang.ByteArray) as Lang.Dictionary<Lang.String,Lang.Object> {
        return {"token" => t, "bytes" => b, "number" => 7, "array" => [1, 2]};
    }
    function scalarMap() as Lang.Dictionary<Lang.String,Lang.Object> { return {"number" => 1, "long" => 2l, "float" => 3.5, "double" => 4.5d, "boolean" => true, "char" => 'A', "string" => "text"}; }
    function nullableMap() as Lang.Dictionary<Lang.String,Lang.Object or Null> { return {"empty" => null, "number" => 1, "string" => "text"}; }
    function emptyMap() as Lang.Dictionary<Lang.String,Lang.Object> { return {}; }
    function heldArray(a as Lang.Array<Lang.Object>) as Lang.WeakReference { var b = [7]b; a.add(b); return b.weak(); }
    function heldMap(d as Lang.Dictionary<Lang.String,Lang.Object>) as Lang.WeakReference { var b = [8]b; d.put("held", b); return b.weak(); }
    function arrays() as Void {
        var values = Objects.constants(); var i = 0;
        while (i < values.size()) { System.println(values[i].toString()); i++; }
        values[0] = "changed"; System.println(Objects.constants()[0].toString());
        System.println(values.addAll([9l, "more"]).size());
        System.println(values.remove("more"));
        System.println(values.indexOf("changed"));
        System.println(values.reverse().slice(0, 1)[0].toString());
        var enums = Objects.enums();
        System.println(enums[0] instanceof Lang.Number); System.println(enums[0].toString());
        System.println(enums[1].toString()); System.println(enums[2].toString());
        var maybe = Objects.optional(); var snapshot = maybe[2]; maybe[2] = null;
        if (snapshot != null) { System.println(snapshot.toString()); }
        System.println(maybe.indexOf(null));
        var sized = Objects.sized(); System.println(sized[0] == null); sized[0] = "filled";
        snapshot = sized[0]; if (snapshot != null) { System.println(snapshot.toString()); }
        var made = Objects.computed(true); System.println(made[1].toString());
        made = Objects.computed(false); System.println(made[1].toString());
        var owned = Objects.empty(); var weak = Objects.heldArray(owned);
        System.println(weak.stillAlive());
        owned.removeAll(owned[0]); System.println(weak.stillAlive());
    }
    function refs() as Void {
        var token = new Token(7); var bytes = [1, 2]b; var array = [1, 2]; var dict = {"one" => 1};
        var method = token.method(:get); var weak = bytes.weak();
        var refs = Objects.references(token, bytes, array, dict, method, weak);
        var saved = refs[0]; if (saved instanceof Token) { System.println(saved.get()); }
        saved = refs[1]; if (saved instanceof Lang.ByteArray) { saved[0] = 9; }
        System.println(bytes[0]);
        System.println(refs[2].equals(array)); System.println(refs[3].equals(dict));
        saved = refs[4];
        if (saved instanceof Lang.Method) {
            var result = saved.invoke();
            if (result != null) { System.println(result.toString()); }
        }
        saved = refs[5]; if (saved instanceof Lang.WeakReference) { System.println(saved.stillAlive()); }
        saved = refs[6]; if (saved instanceof Lang.Symbol) { System.println(saved.toString()); }
    }
    function maps() as Void {
        var scalars = Objects.scalarMap();
        System.println(scalars.values().size());
        var n = scalars["long"]; if (n != null) { System.println(n.toString()); }
        var token = new Token(11); var bytes = [5]b; var map = Objects.mapping(token, bytes);
        var value = map["token"]; if (value instanceof Token) { System.println(value.get()); }
        value = map["bytes"]; if (value instanceof Lang.ByteArray) { value[0] = 6; }
        System.println(bytes[0]);
        value = map["array"]; if (value != null) { System.println(value.toString()); }
        System.println(map["missing"] == null);
        var vals = map.values(); System.println(vals.indexOf(token) >= 0);
        map.put("token", "replacement"); value = map["token"]; if (value != null) { System.println(value.toString()); }
        var maybe = Objects.nullableMap(); var saved = maybe["string"]; maybe["string"] = null;
        if (saved != null) { System.println(saved.toString()); }
        System.println(maybe.values().indexOf(null) >= 0);
        var held = Objects.emptyMap(); var weak = Objects.heldMap(held);
        var enumeration = held.values(); held.remove("held");
        System.println(weak.stillAlive());
        enumeration.removeAll(enumeration[0]); System.println(weak.stillAlive());
    }
    function run() as Void { Objects.arrays(); Objects.refs(); Objects.maps(); }
}
class MinimalConsoleApp extends Toybox.Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Objects.run(); System.exit(); }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
