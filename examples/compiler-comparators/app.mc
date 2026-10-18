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
