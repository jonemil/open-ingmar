using Toybox.Lang;
using Toybox.System;
class MinimalConsoleApp extends Toybox.Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { ByteProgram.run(); }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
class ByteBox {
    var fixed as Lang.ByteArray;
    var maybe as Lang.ByteArray or Null = null;
    function initialize(bytes as Lang.ByteArray) { self.fixed = bytes; }
    function bytes() as Lang.ByteArray { return self.fixed; }
    function optional() as Lang.ByteArray or Null { return self.maybe; }
    function replace(bytes as Lang.ByteArray) as Void { self.fixed = bytes; }
    function save(bytes as Lang.ByteArray or Null) as Void { self.maybe = bytes; }
    function change() as Void { self.fixed[0] += 1; }
}
module Bytes {
    function make(n as Toybox.Lang.Number) as Lang.ByteArray { return new[n]b; }
    function echo(b as Lang.ByteArray) as Lang.ByteArray { return b; }
    function effect(n as Toybox.Lang.Number) as Toybox.Lang.Number { System.println(n); return n; }
    function choose(flag as Lang.Boolean) as Lang.ByteArray {
        return flag ? new[Bytes.effect(2)]b : new[Bytes.effect(4)]b;
    }
    function literal() as Lang.ByteArray {
        return [0, 1, 127, 128, 255, -1, -128, 'A']b;
    }
    function empty() as Lang.ByteArray { return []b; }
    function straight(n as Toybox.Lang.Number) as Lang.ByteArray { return [n, n + 1]b; }
    function computed(n as Toybox.Lang.Number) as Lang.ByteArray {
        return [Bytes.effect(n), true ? Bytes.effect(n + 1) : Bytes.effect(99), 'Z']b;
    }
    function optional(flag as Lang.Boolean) as Lang.ByteArray or Null {
        return flag ? [7, 8]b : null;
    }
    function echoOptional(b as Lang.ByteArray or Null) as Lang.ByteArray or Null {
        return b;
    }
    function required(b as Lang.ByteArray) as Toybox.Lang.Number { return b.size(); }
    function describe(b as Lang.ByteArray or Null) as Void {
        if (b == null) { System.println("no bytes"); return; }
        b[0] += 1;
        System.println(b[0]);
        System.println(Bytes.required(b));
    }
    function list(flag as Lang.Boolean) as Lang.Array<Lang.ByteArray> {
        return [[1, 2]b, flag ? [3]b : [4]b];
    }
    function emptyList() as Lang.Array<Lang.ByteArray> { return []; }
    function nullableList(flag as Lang.Boolean) as Lang.Array<Lang.ByteArray or Null> {
        return [null, flag ? [7]b : null];
    }
    function nullList() as Lang.Array<Lang.ByteArray or Null> { return [null]; }
    function slots(n as Toybox.Lang.Number) as Lang.Array<Lang.ByteArray or Null> { return new[n]; }
    function table(flag as Lang.Boolean) as Lang.Dictionary<Lang.String, Lang.ByteArray> {
        return {"one" => [13]b, "two" => flag ? [14]b : [15]b};
    }
    function emptyTable() as Lang.Dictionary<Lang.String, Lang.ByteArray> { return {}; }
    function optionalTable() as Lang.Dictionary<Lang.String, Lang.ByteArray or Null> {
        return {"empty" => null, "live" => Bytes.optional(true)};
    }
    function nullTable() as Lang.Dictionary<Lang.String, Lang.ByteArray or Null> {
        return {"empty" => null};
    }
    function values(table as Lang.Dictionary<Lang.String, Lang.ByteArray>)
        as Lang.Array<Lang.ByteArray> { return table.values(); }
    function codec(value as Lang.Numeric, format as Lang.NumberFormat) as Void {
        var b = new[6]b;
        b.encodeNumber(value, format, {:offset => 1, :endianness => Lang.ENDIAN_BIG});
        var decoded = b.decodeNumber(format, {:offset => 1, :endianness => Lang.ENDIAN_BIG});
        System.println(b.toString());
        System.println(decoded.toString());
        System.println(decoded instanceof Lang.Number);
        System.println(decoded instanceof Lang.Long);
        System.println(decoded instanceof Lang.Float);
        if (decoded instanceof Lang.Number) { System.println(decoded + 1); }
    }
    function byteKeys(key as Lang.ByteArray)
        as Lang.Dictionary<Lang.ByteArray, Lang.Number> { return {key => 10}; }
    function recovered(o as Lang.Object or Null) as Toybox.Lang.Number {
        if (!(o instanceof Lang.ByteArray)) { return -1; }
        o[0] += 1;
        return o[0];
    }
    function assertion(o as Lang.Object or Null) as Lang.ByteArray or Null {
        return o as Lang.ByteArray or Null;
    }
}
module ByteProgram {
    function run() as Void {
        var a = Bytes.make(Bytes.effect(3));
        System.println(a.toString());
        System.println(a[0]);
        a[0] = -1;
        a[1] = 'A';
        a[2] += 7;
        System.println(a.toString());
        System.println(a[0]);
        var shared = Bytes.echo(a);
        System.println(shared.equals(a));
        System.println(a.equals([255, 65, 7]));
        var b = new[0]b;
        b.addAll(a);
        System.println(b.equals(a));
        b[0] = 1;
        System.println(a[0]);
        b.add('B');
        b.addAll([3, 4]);
        b.addAll(['C']);
        System.println(b.toString());
        System.println(b.indexOf('A'));
        System.println(b.remove(7));
        b.add(1);
        System.println(b.removeAll(1));
        System.println(b.toString());
        var reversed = b.reverse();
        System.println(reversed.toString());
        reversed[0] = 8;
        System.println(b.toString());
        var sliced = b.slice(1, -1);
        System.println(sliced.toString());
        sliced[0] = 9;
        System.println(b.toString());
        System.println(b.slice(null, null).equals(b));
        System.println(b.hashCode() == b.hashCode());
        var callback = b.method(:size);
        var size = callback.invoke();
        if (size != null) { System.println(size.toString()); }
        var weak = b.weak();
        System.println(weak.stillAlive());
        var got = weak.get();
        if (got != null) { System.println(got.equals(b)); }
        System.println(Bytes.choose(true).size());
        System.println(Bytes.choose(false).size());
        a = new[true ? Bytes.effect(1) : Bytes.effect(9)]b;
        System.println(a.size());
        System.println(shared.toString());
        System.println(Bytes.literal().toString());
        var first = Bytes.literal();
        var second = Bytes.literal();
        first[0] = 42;
        System.println(second[0]);
        var empty = Bytes.empty();
        var anotherEmpty = Bytes.empty();
        empty.add(7);
        System.println(anotherEmpty.size());
        System.println(Bytes.straight(4).toString());
        System.println(Bytes.computed(10).toString());
        System.println(Bytes.echo([-1, 'A']b).toString());
        b.addAll([]b);
        System.println(b.size());
        var picked = true ? [Bytes.effect(3)]b : [Bytes.effect(9)]b;
        System.println(picked.toString());
        Bytes.describe(Bytes.echoOptional(Bytes.optional(true)));
        Bytes.describe(Bytes.optional(false));
        var maybe = Bytes.optional(true);
        System.println(maybe != null && maybe.size() > 0);
        System.println(maybe != null ? maybe[1] : 0);
        var saved = Bytes.echoOptional(maybe);
        if (maybe != null) {
            var watched = maybe.weak();
            System.println(watched.stillAlive());
            maybe = null;
            System.println(watched.stillAlive());
            saved = null;
            System.println(watched.stillAlive());
        }
        maybe = Bytes.optional(true);
        while (maybe != null) {
            System.println(Bytes.required(maybe));
            maybe = null;
        }
        System.println(maybe == null);
        var box = new ByteBox([20, 21]b);
        System.println(box.optional() == null);
        var original = box.bytes();
        box.change();
        System.println(original[0]);
        box.replace([30]b);
        System.println(original[0]);
        System.println(box.bytes()[0]);
        box.save(Bytes.optional(true));
        var snapshot = box.optional();
        if (snapshot != null) {
            var fieldWatch = snapshot.weak();
            snapshot = null;
            System.println(fieldWatch.stillAlive());
            box.save(null);
            System.println(fieldWatch.stillAlive());
        }
        var list = Bytes.list(true);
        System.println(list.size());
        var element = list[0];
        element[0] = 11;
        System.println(list[0][0]);
        var reversedList = list.reverse();
        reversedList[1][1] = 12;
        System.println(list[0].toString());
        var copiedList = list.slice(null, null);
        System.println(copiedList.equals(list));
        copiedList[0] = [99]b;
        System.println(list[0][0]);
        var emptyList = Bytes.emptyList();
        emptyList.addAll(list);
        emptyList.add([5]b);
        System.println(emptyList.size());
        System.println(emptyList.remove(element));
        System.println(emptyList.size());
        var nullableList = Bytes.nullableList(true);
        System.println(nullableList[0] == null);
        var nullableElement = nullableList[1];
        if (nullableElement != null) { System.println(nullableElement[0]); }
        nullableList[1] = null;
        System.println(nullableElement != null);
        System.println(Bytes.nullList()[0] == null);
        var slots = Bytes.slots(2);
        System.println(slots[0] == null);
        slots[1] = Bytes.optional(true);
        var retained = slots[1];
        if (retained != null) {
            var elementWatch = retained.weak();
            retained = null;
            System.println(elementWatch.stillAlive());
            slots[1] = null;
            System.println(elementWatch.stillAlive());
        }
        var table = Bytes.table(true);
        System.println(table.size());
        System.println(table["missing"] == null);
        var item = table.get("one");
        if (item != null) {
            item[0] += 1;
            var again = table["one"];
            if (again != null) { System.println(again[0]); }
            table.put("one", [20]b);
            System.println(item[0]);
        }
        var values = Bytes.values(table);
        System.println(values.size());
        table.remove("one");
        table.remove("two");
        System.println(values.size());
        var emptyTable = Bytes.emptyTable();
        emptyTable["saved"] = values[0];
        System.println(emptyTable.hasKey("saved"));
        emptyTable.remove("saved");
        System.println(emptyTable.isEmpty());
        var optionalTable = Bytes.optionalTable();
        System.println(optionalTable.hasKey("empty"));
        System.println(optionalTable.get("empty") == null);
        var dictSnapshot = optionalTable["live"];
        if (dictSnapshot != null) {
            var dictWatch = dictSnapshot.weak();
            var dictValues = optionalTable.values();
            dictSnapshot = null;
            optionalTable.remove("live");
            System.println(dictWatch.stillAlive());
            dictValues = [];
            System.println(dictWatch.stillAlive());
        }
        optionalTable.put("empty", [6]b);
        optionalTable["empty"] = null;
        System.println(optionalTable.keys().size());
        System.println(Bytes.nullTable()["empty"] == null);
        var concrete = Bytes.echo([40, 41]b);
        var recoveryWatch = concrete.weak();
        var erased = recoveryWatch.get();
        System.println(Bytes.recovered(erased));
        System.println(Bytes.recovered(null));
        System.println(Bytes.recovered([1]));
        if (erased instanceof Lang.ByteArray) { System.println(erased.size()); }
        var asserted = Bytes.assertion(recoveryWatch.get());
        if (asserted != null) { System.println(asserted[0]); }
        var concreteAssertion = recoveryWatch.get() as Lang.ByteArray;
        System.println(concreteAssertion.size());
        System.println(Bytes.assertion(null) == null);
        var reversedCallback = concrete.method(:reverse);
        var erasedResult = reversedCallback.invoke();
        if (erasedResult instanceof Lang.ByteArray) {
            System.println(erasedResult.toString());
        }
        ByteProgram.finalChecks();
        System.exit();
    }
    function finalChecks() as Void {
        Bytes.codec(1.5, Lang.NUMBER_FORMAT_FLOAT);
        Bytes.codec(-1234, Lang.NUMBER_FORMAT_SINT16);
        Bytes.codec(-123456, Lang.NUMBER_FORMAT_SINT32);
        Bytes.codec(-12, Lang.NUMBER_FORMAT_SINT8);
        Bytes.codec(54321, Lang.NUMBER_FORMAT_UINT16);
        Bytes.codec(4294967295l, Lang.NUMBER_FORMAT_UINT32);
        Bytes.codec(250, Lang.NUMBER_FORMAT_UINT8);
        var defaultBytes = new[4]b;
        defaultBytes.encodeNumber(258, Lang.NUMBER_FORMAT_UINT16, {});
        System.println(defaultBytes.toString());
        System.println(defaultBytes.decodeNumber(Lang.NUMBER_FORMAT_UINT16, {}).toString());
        defaultBytes.encodeNumber(2.5d, Lang.NUMBER_FORMAT_FLOAT, {});
        System.println(defaultBytes.decodeNumber(Lang.NUMBER_FORMAT_FLOAT, {}).toString());
        var byteKey = Bytes.echo([1, 2]b);
        var equalKey = Bytes.echo([1, 2]b);
        var byteTable = Bytes.byteKeys(byteKey);
        System.println(byteKey.equals(equalKey));
        System.println(byteKey.hashCode() == equalKey.hashCode());
        System.println(byteTable.get(equalKey) == null);
        byteTable.put(equalKey, 20);
        System.println(byteTable.size());
        var found = byteTable.get(byteKey);
        if (found != null) { System.println(found); }
        var keyHash = byteKey.hashCode();
        byteKey[0] = 9;
        System.println(byteKey.hashCode() == keyHash);
        found = byteTable[byteKey];
        if (found != null) { System.println(found); }
        System.println(byteTable.get([9, 2]b) == null);
        var enumeration = byteTable.keys();
        byteTable.remove(byteKey);
        System.println(enumeration.size());
        System.println(byteTable.size());
        var ownedKey = Bytes.optional(true);
        if (ownedKey != null) {
            var ownedTable = Bytes.byteKeys(ownedKey);
            var watch = ownedKey.weak();
            var keptKeys = ownedTable.keys();
            ownedTable.remove(ownedKey);
            ownedKey = null;
            System.println(watch.stillAlive());
            keptKeys = [];
            System.println(watch.stillAlive());
        }
    }
}
