using Toybox.Application;
using Toybox.System;
module Comparators {
    function array(a as Toybox.Lang.Array<Toybox.Lang.String>) as Toybox.Lang.Array<Toybox.Lang.String> {
        System.println("array");
        return a;
    }
    function comparator(c as Ordering) as Ordering {
        System.println("comparator");
        return c;
    }
    function sort(a as Toybox.Lang.Array<Toybox.Lang.String>, c as Ordering) as Void { a.sort(c); }
    function empty() as Toybox.Lang.Array<Toybox.Lang.String> { return []; }
    function run() as Void {
        InterfaceRefinements.run();
        InterfaceConstants.run();
        InterfaceKeys.run();
        InterfaceDictionaries.run();
        InterfaceSorting.run();
        InterfaceArrays.run();
        InterfaceFields.run();
        NullableInterfaces.run();
        NativeComparables.run();
        InterfaceCalls.run();
        var text = new TextOrder();
        System.println(text.compareTo("a") > 0);
        System.println(text.compareTo("b") == 0);
        System.println(text.compareTo("c") < 0);
        var ascending = new Ordering(1);
        var descending = new Ordering(-1);
        var a = ["bb", "a", "aa", "b", "a"];
        var shared = a;
        Comparators.array(a).sort(Comparators.comparator(descending));
        System.println(shared.toString());
        System.println(descending.calls() > 0);
        Comparators.sort(a, ascending);
        System.println(a.toString());
        System.println(ascending.calls() > 0);
        var numbers = [2, 10, 1];
        numbers.sort(descending);
        System.println(numbers.toString());
        var empty = Comparators.empty();
        empty.sort(ascending);
        System.println(empty.size());
        var singleton = ["only"];
        singleton.sort(descending);
        System.println(singleton[0]);
    }
}
module InterfaceConstants {
    function values() as Toybox.Lang.Array<Toybox.Lang.Comparable> {return [97, 'a', true, 1l, 1.0, 1.0d, "a"];}
    function optional() as Toybox.Lang.Array<Toybox.Lang.Comparable or Null> {return [null, 97, 'a'];}
    function orders() as Toybox.Lang.Array<Toybox.Lang.Comparator or Null> {return [null];}
    function run() as Void {
        var values = InterfaceConstants.values();
        System.println(values[0] instanceof Toybox.Lang.Number);
        System.println(values[1] instanceof Toybox.Lang.Char);
        System.println(values[2] instanceof Toybox.Lang.Boolean);
        System.println(values[3] instanceof Toybox.Lang.Long);
        System.println(values[4] instanceof Toybox.Lang.Float);
        System.println(values[5] instanceof Toybox.Lang.Double);
        System.println(values[6] instanceof Toybox.Lang.String);
        System.println(values[0].compareTo(97) == 0);
        var separate = InterfaceConstants.values();
        var shared = values;
        values[0] = false;
        System.println(shared[0] instanceof Toybox.Lang.Boolean);
        System.println(separate[0] instanceof Toybox.Lang.Number);
        var optional = InterfaceConstants.optional();
        System.println(optional[0] == null);
        System.println(optional[1] instanceof Toybox.Lang.Number);
        System.println(optional[2] instanceof Toybox.Lang.Char);
        var snapshot = optional[1];
        if (snapshot != null) {System.println(snapshot.compareTo(97) == 0);}
        var names = ["b", "a"];
        names.sort(InterfaceConstants.orders()[0]);
        System.println(names.toString());
    }
}
module InterfaceKeys {
    function values(key as Toybox.Lang.Comparable) as Toybox.Lang.Dictionary<Toybox.Lang.Comparable,Toybox.Lang.Number> {
        return {key => 10};
    }
    function orders(key as Toybox.Lang.Comparator) as Toybox.Lang.Dictionary<Toybox.Lang.Comparator,Toybox.Lang.Comparable or Null> {
        return {key => new TextOrder()};
    }
    function empty() as Toybox.Lang.Dictionary<Toybox.Lang.Comparable,Toybox.Lang.Number> {return {};}
    function nativeKeys() as Toybox.Lang.Dictionary<Toybox.Lang.Comparable,Toybox.Lang.Number> {
        var keys = InterfaceKeys.empty();
        keys.put(97, 1);
        keys.put('a', 2);
        keys.put("a", 3);
        keys.put(true, 4);
        keys.put(1l, 5);
        keys.put(1.0, 6);
        keys.put(1.0d, 7);
        return keys;
    }
    function run() as Void {
        var first = new HashKey(1);
        var equal = new HashKey(1);
        var different = new HashKey(2);
        var values = InterfaceKeys.values(first);
        var initial = values[first];
        if (initial != null) { System.println(initial); }
        var matched = values[equal];
        if (matched != null) { System.println(matched); }
        values.put(equal, 20);
        System.println(values.size());
        var replaced = values.get(first);
        if (replaced != null) { System.println(replaced); }
        values[different] = 30;
        System.println(values.size());
        System.println(values.hasKey(new HashKey(2)));
        System.println(values.hasKey(new HashKey(3)));
        System.println(first.hashCalls() > 0);
        System.println(first.equalCalls() + equal.equalCalls() + different.equalCalls() > 0);
        System.println(first.comparisonCalls());
        System.println(equal.comparisonCalls());
        var keys = values.keys();
        System.println(keys.size());
        System.println(keys[0] instanceof HashKey);
        values.remove(new HashKey(1));
        System.println(values.hasKey(first));
        System.println(keys.size());
        var orders = InterfaceKeys.orders(first);
        var found = orders[equal];
        if (found != null) { System.println(found.compareTo("b") == 0); }
        orders.put(equal, null);
        System.println(orders.size());
        System.println(orders[first] == null);
        var plainKey = new TextOrder();
        var plain = InterfaceKeys.values(plainKey);
        System.println(plain.hasKey(plainKey));
        System.println(!plain.hasKey(new TextOrder()));
        var plainOrder = new Ordering(1);
        var plainOrders = InterfaceKeys.orders(plainOrder);
        System.println(plainOrders.hasKey(plainOrder));
        System.println(!plainOrders.hasKey(new Ordering(1)));
        var nativeKeys = InterfaceKeys.nativeKeys();
        System.println(nativeKeys.size());
        System.println(nativeKeys.hasKey(97));
        System.println(nativeKeys.hasKey('a'));
        System.println(nativeKeys.hasKey("a"));
        System.println(nativeKeys.hasKey(true));
        System.println(nativeKeys.hasKey(1l));
        System.println(nativeKeys.hasKey(1.0));
        System.println(nativeKeys.hasKey(1.0d));
        var number = nativeKeys[97];
        if (number != null) {System.println(number);}
        var character = nativeKeys['a'];
        if (character != null) {System.println(character);}
    }
}
module InterfaceDictionaries {
    function values() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Comparable> {
        return {"a" => new TextOrder(), "b" => 1};
    }
    function scalars() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Comparable> {
        return {"n" => 97, "c" => 'a', "s" => "a"};
    }
    function optional() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Comparable or Null> {
        return {"a" => null, "b" => new TextOrder()};
    }
    function orders() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Comparator> {
        return {"a" => new Ordering(-1)};
    }
    function optionalOrders() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Comparator or Null> {
        return {"a" => null, "b" => new Ordering(1)};
    }
    function empty() as Toybox.Lang.Dictionary<Toybox.Lang.String, Toybox.Lang.Comparable> { return {}; }
    function run() as Void {
        var values = InterfaceDictionaries.values();
        var shared = values;
        var item = values["a"];
        if (item != null) { System.println(item.compareTo("b") == 0); }
        System.println(values["missing"] == null);
        values["a"] = new TextOrder();
        values.put("c", new TextOrder());
        System.println(shared.size());
        System.println(values.hasKey("c"));
        var snapshot = values.values();
        values.remove("c");
        System.println(snapshot.size());
        System.println(shared.size());
        var empty = InterfaceDictionaries.empty();
        empty.put("key", new TextOrder());
        var found = empty.get("key");
        if (found != null) { System.println(found.compareTo("b") == 0); }
        System.println(empty.keys().toString());
        var scalars = InterfaceDictionaries.scalars();
        System.println(scalars["n"] instanceof Toybox.Lang.Number);
        System.println(scalars["c"] instanceof Toybox.Lang.Char);
        var optional = InterfaceDictionaries.optional();
        System.println(optional["a"] == null);
        optional["a"] = new TextOrder();
        optional.put("b", null);
        var nullable = optional.values();
        System.println(nullable.size());
        System.println(optional["b"] == null);
        var orders = InterfaceDictionaries.orders();
        var order = orders["a"];
        if (order != null) { System.println(order.compare("a", "b") > 0); }
        orders["a"] = new Ordering(1);
        System.println(orders.values()[0].compare("a", "b") < 0);
        var optionalOrders = InterfaceDictionaries.optionalOrders();
        var names = ["b", "a"];
        names.sort(optionalOrders["a"]);
        System.println(names.toString());
        names.sort(optionalOrders["b"]);
        System.println(names.toString());
    }
}
module InterfaceSorting {
    function values() as Toybox.Lang.Array<Toybox.Lang.Comparable> {
        return [new Ranked(3), new Ranked(1), new Ranked(2), new Ranked(1)];
    }
    function numbers() as Toybox.Lang.Array<Toybox.Lang.Comparable> { return [3, 1, 2]; }
    function names() as Toybox.Lang.Array<Toybox.Lang.Comparable> { return ["c", "a", "b"]; }
    function orders() as Toybox.Lang.Array<Toybox.Lang.Comparator> { return [new Ranked(1), new Ranked(3), new Ranked(2)]; }
    function sort(a as Toybox.Lang.Array<Toybox.Lang.Comparable>, c as Toybox.Lang.Comparator or Null) as Void { a.sort(c); }
    function comparator(value as Toybox.Lang.Comparator) as Toybox.Lang.Comparator { return value; }
    function run() as Void {
        var values = InterfaceSorting.values();
        var shared = values;
        values.sort(null);
        System.println((shared[0] as Ranked).value());
        System.println((shared[3] as Ranked).value());
        var order = new RankOrder();
        values.sort(order);
        System.println((shared[0] as Ranked).value());
        System.println((shared[3] as Ranked).value());
        System.println(order.calls() > 0);
        InterfaceSorting.sort(values, null);
        System.println((shared[0] as Ranked).value());
        InterfaceSorting.sort(values, order);
        System.println((shared[0] as Ranked).value());
        var numbers = InterfaceSorting.numbers();
        numbers.sort(null);
        System.println(numbers.toString());
        var names = InterfaceSorting.names();
        names.sort(null);
        System.println(names.toString());
        var empty = InterfaceArrays.empty();
        empty.sort(null);
        System.println(empty.size());
        var orders = InterfaceSorting.orders();
        orders.sort(order);
        System.println((orders[0] as Ranked).value());
        var checkedOrder = InterfaceSorting.comparator(order);
        orders.sort(checkedOrder);
        System.println((orders[2] as Ranked).value());
    }
}
module InterfaceArrays {
    function values() as Toybox.Lang.Array<Toybox.Lang.Comparable> { return [new TextOrder(), 3, "a"]; }
    function nullable() as Toybox.Lang.Array<Toybox.Lang.Comparable or Null> { return [null, new TextOrder(), 1]; }
    function orders() as Toybox.Lang.Array<Toybox.Lang.Comparator> { return [new Ordering(-1)]; }
    function optionalOrders() as Toybox.Lang.Array<Toybox.Lang.Comparator or Null> { return [null, new Ordering(1)]; }
    function empty() as Toybox.Lang.Array<Toybox.Lang.Comparable> { return []; }
    function run() as Void {
        var values = InterfaceArrays.values();
        System.println(values[0].compareTo("b") == 0);
        var shared = values;
        values[0] = new TextOrder();
        System.println(values.indexOf(values[0]) == 0);
        values.add(new TextOrder());
        values.addAll([true, 'a', 1l, 1.0, 1.0d]);
        System.println(shared.size());
        System.println(values[5] instanceof Toybox.Lang.Char);
        System.println(values.remove(values[3]));
        values.reverse();
        System.println(values.slice(0, 2).size());
        var object = new TextOrder();
        var empty = InterfaceArrays.empty();
        empty.add(object);
        empty.add(object);
        System.println(empty.indexOf(object) == 0);
        System.println(empty.removeAll(object));
        System.println(empty.size());
        var optional = InterfaceArrays.nullable();
        optional[1] = null;
        System.println(optional[1] == null);
        optional.add(new TextOrder());
        var item = optional[3];
        if (item != null) { System.println(item.compareTo("b") == 0); }
        var orders = InterfaceArrays.orders();
        System.println(orders[0].compare("a", "b") > 0);
        orders[0] = new Ordering(1);
        System.println(orders[0].compare("a", "b") < 0);
        var optionalOrders = InterfaceArrays.optionalOrders();
        var names = ["b", "a"];
        names.sort(optionalOrders[0]);
        System.println(names.toString());
    }
}
module InterfaceFields {
    function run() as Void {
        var holder = new InterfaceHolder("b");
        System.println(holder.compareValue("b") == 0);
        System.println(holder.compareMaybe("x"));
        holder.setMaybe(new TextOrder());
        System.println(holder.compareMaybe("b") == 0);
        System.println(holder.takeMaybe("b") == 0);
        System.println(holder.compareMaybe("b"));
        holder.replace(3);
        System.println(holder.compareValue(2) > 0);
        holder.reset();
        System.println(holder.compareValue("b") == 0);
        System.println(holder.sorted());
        holder.select(new Ordering(-1));
        System.println(holder.sorted());
        holder.select(null);
        System.println(holder.sorted());
        System.println(holder.requiredSorted());
        var other = new InterfaceHolder("a");
        System.println(other.compareValue("a") == 0);
        System.println(holder.compareValue("b") == 0);
    }
}
module NullableInterfaces {
    function comparable(present as Toybox.Lang.Boolean) as Toybox.Lang.Comparable or Null {
        if (present) { return new TextOrder(); } else { return null; }
    }
    function comparator(present as Toybox.Lang.Boolean) as Toybox.Lang.Comparator or Null {
        if (present) { return new Ordering(-1); } else { return null; }
    }
    function read(value as Toybox.Lang.Comparable or Null) as Toybox.Lang.Number {
        if (value == null) { return -9; }
        return value.compareTo("b");
    }
    function sort(order as Toybox.Lang.Comparator or Null) as Toybox.Lang.String {
        var values = ["a", "c", "b"];
        values.sort(order);
        return values.toString();
    }
    function run() as Void {
        System.println(NullableInterfaces.read(null));
        System.println(NullableInterfaces.read(NullableInterfaces.comparable(true)) == 0);
        var value = NullableInterfaces.comparable(false);
        value = new TextOrder();
        if (value != null) { System.println(value.compareTo("a") > 0); }
        value = null;
        System.println(value == null);
        value = 3;
        if (value != null) { System.println(value.compareTo(3) == 0); }
        while (value != null) { System.println(value.compareTo(3) == 0); value = null; }
        System.println(NullableInterfaces.sort(NullableInterfaces.comparator(false)));
        System.println(NullableInterfaces.sort(NullableInterfaces.comparator(true)));
        var order = NullableInterfaces.comparator(false);
        order = new Ordering(-1);
        if (order != null) { System.println(order.compare("a", "b") > 0); }
        order = null;
        System.println(order == null);
        var exact = InterfaceCalls.create();
        exact = new TextOrder();
        System.println(exact.compareTo("b") == 0);
        var selected = true ? InterfaceCalls.create() : null;
        System.println(NullableInterfaces.read(selected) == 0);
    }
}
module NativeComparables {
    function equal(value as Toybox.Lang.Comparable, other as Toybox.Lang.Object) as Toybox.Lang.Boolean {
        return value.compareTo(other) == 0;
    }
    function create() as Toybox.Lang.Comparable { return 3; }
    function guarded(value as Toybox.Lang.Object or Null) as Toybox.Lang.Boolean {
        if (value instanceof Toybox.Lang.Number) { return NativeComparables.equal(value, 3); }
        else { return false; }
    }
    function run() as Void {
        System.println(NativeComparables.equal(3, 3));
        System.println(NativeComparables.equal(3l, 3l));
        System.println(NativeComparables.equal(3.0, 3.0));
        System.println(NativeComparables.equal(3.0d, 3.0d));
        System.println(NativeComparables.equal(true, true));
        System.println(NativeComparables.equal('b', 'b'));
        System.println(NativeComparables.equal("b", "b"));
        var value = NativeComparables.create();
        System.println(value.compareTo(2l) > 0);
        value = "b";
        System.println(value.compareTo("a") > 0);
        var selected = true ? value : "c";
        System.println(selected.compareTo("b") == 0);
        var reversed = false ? 1 : selected;
        System.println(reversed.compareTo("b") == 0);
        System.println(NativeComparables.guarded(3));
        System.println(!NativeComparables.guarded(null));
    }
}
module InterfaceCalls {
    function comparable(value as Toybox.Lang.Comparable) as Toybox.Lang.Comparable { return value; }
    function create() as Toybox.Lang.Comparable { return new TextOrder(); }
    function comparator(value as Ordering) as Toybox.Lang.Comparator { return value; }
    function forward(value as Toybox.Lang.Comparator) as Toybox.Lang.Comparator { return value; }
    function receiver(value as Toybox.Lang.Comparable) as Toybox.Lang.Comparable {
        System.println("interface-receiver"); return value;
    }
    function argument() as Toybox.Lang.Object { System.println("interface-argument"); return "a"; }
    function run() as Void {
        var text = InterfaceCalls.create();
        var shared = InterfaceCalls.comparable(text);
        System.println(shared.compareTo("b") == 0);
        System.println(InterfaceCalls.receiver(text).compareTo(InterfaceCalls.argument()) > 0);
        shared = InterfaceCalls.comparable(new TextOrder());
        System.println(shared.compareTo("c") < 0);
        var order = InterfaceCalls.comparator(new Ordering(-1));
        order = InterfaceCalls.forward(order);
        System.println(order.compare("a", "b") > 0);
        var values = ["a", "c", "b"];
        values.sort(order);
        System.println(values.toString());
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Comparators.run(); System.exit(); }
}

module InterfaceRefinements {
    function accept(c as Toybox.Lang.Comparable, other as Toybox.Lang.Object) as Toybox.Lang.Number {
        return c.compareTo(other);
    }
    function check(c as Toybox.Lang.Comparable or Null) as Toybox.Lang.Boolean {
        if (c instanceof Toybox.Lang.Number) {
            return c + 1 == 42 && InterfaceRefinements.accept(c, c) == 0;
        }
        if (c instanceof Toybox.Lang.Long) {
            return c + 1l == 42l && InterfaceRefinements.accept(c, c) == 0;
        }
        if (c instanceof Toybox.Lang.Float) {
            return c + 1.0 == 42.0 && InterfaceRefinements.accept(c, c) == 0;
        }
        if (c instanceof Toybox.Lang.Double) {
            return c + 1.0d == 42.0d && InterfaceRefinements.accept(c, c) == 0;
        }
        if (c instanceof Toybox.Lang.Boolean) {
            return !c && InterfaceRefinements.accept(c, false) == 0;
        }
        if (c instanceof Toybox.Lang.Char) {
            return c.toNumber() == 65 && InterfaceRefinements.accept(c, 'A') == 0;
        }
        if (c instanceof Toybox.Lang.String) {
            return c.length() == 2 && InterfaceRefinements.accept(c, "hi") == 0;
        }
        return c == null;
    }
    function rewrite(c as Toybox.Lang.Comparable) as Toybox.Lang.Number {
        var value = c;
        var sum = 0;
        while (value instanceof Toybox.Lang.Number) {
            sum = value + 1;
            value = "done";
        }
        return sum;
    }
    function values() as Toybox.Lang.Array<Toybox.Lang.Comparable> { return [41]; }
    function run() as Void {
        System.println(InterfaceRefinements.check(41));
        System.println(InterfaceRefinements.check(41l));
        System.println(InterfaceRefinements.check(41.0));
        System.println(InterfaceRefinements.check(41.0d));
        System.println(InterfaceRefinements.check(false));
        System.println(InterfaceRefinements.check('A'));
        System.println(InterfaceRefinements.check("hi"));
        System.println(InterfaceRefinements.check(null));
        System.println(InterfaceRefinements.rewrite(41));
        var values = InterfaceRefinements.values();
        var snapshot = values[0];
        if (snapshot instanceof Toybox.Lang.Number) {
            values[0] = "changed";
            System.println(snapshot + 1);
        }
    }
}
