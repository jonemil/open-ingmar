using Toybox.Application;
using Toybox.Lang;
using Toybox.System;

typedef Row as Lang.Array<Lang.Number>;
typedef Rows as Lang.Array<Row>;
typedef Cell as [Lang.Number, Lang.String];
typedef Entry as Lang.Dictionary<Lang.String, Lang.Number>;
typedef Entries as Lang.Array<Entry>;

class StableOrder {
    function compare(a as Lang.Object, b as Lang.Object) as Lang.Number { return 0; }
}
module NestedArrays {
    function cells() as Lang.Array<Cell> { return [[7, "tuple"], [8, "more"]]; }
    function rows() as Rows { return [[1, 2], [], [3]]; }
    function entries() as Entries { return [{"a" => 7}, {}]; }
    function optional(flag as Lang.Boolean) as Lang.Array<Row or Null> {
        return [flag ? [4] : null, []];
    }
    function allocated() as Lang.Array<Row or Null> { return new[2]; }
    function deep() as Lang.Array<Lang.Array<Row>> { return [[[5]], [[]]]; }
    function append(x as [Row], tail as Rows) as Rows { x.addAll(tail); return x; }
    function write(x as [Row], i as Lang.Number) as Rows { x[i] = [8]; return x; }
    function marker(n as Lang.Number) as Lang.Number { System.println(n); return n; }
    function ordered() as Rows {
        return [[NestedArrays.marker(10)], [NestedArrays.marker(11)]];
    }
    function run() as Void {
        var rows = NestedArrays.rows();
        System.println(rows[0][1]);
        var firstRow = rows[0];
        var copy = rows.reverse();
        copy[2][0] = 9;
        System.println(rows[0][0]);
        copy[2] = [6];
        System.println(rows[0][0]);
        var slice = rows.slice(0, 1);
        slice[0][1] = 12;
        System.println(rows[0][1]);
        rows.add([]).addAll([[4]]);
        System.println(rows.size());
        System.println(rows.indexOf(firstRow));
        System.println(rows.remove(firstRow));
        System.println(rows.size());
        System.println(rows.removeAll(firstRow));
        System.println(rows.size());
        rows.sort(new StableOrder());
        System.println(rows.size());
        var present = NestedArrays.optional(true)[1];
        if (present != null) { System.println(present.size()); }
        var absent = NestedArrays.optional(false)[0];
        System.println(absent == null);
        System.println(NestedArrays.allocated()[1] == null);
        System.println(NestedArrays.deep()[0][0][0]);
        System.println(NestedArrays.cells()[0][1]);
        System.println(NestedArrays.append([[1]], [[2]]).size());
        System.println(NestedArrays.write([[1]], 0)[0][0]);
        var maps = NestedArrays.entries();
        maps[0]["a"] = 13;
        var value = maps.reverse()[1]["a"];
        if (value != null) { System.println(value); }
        maps.add({}).addAll([{"b" => 14}]);
        System.println(maps.size());
        System.println(maps.indexOf(maps[0]));
        System.println(maps.remove(maps[0]));
        System.println(maps.size());
        var duplicate = maps[0];
        maps.add(duplicate);
        System.println(maps.removeAll(duplicate));
        System.println(maps.size());
        System.println(NestedArrays.ordered()[1][0]);
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { NestedArrays.run(); System.exit(); }
}
