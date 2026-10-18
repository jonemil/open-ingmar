using Toybox.Lang;
using Toybox.Application;
using Toybox.System;
module Bytes {
    function literal() as Lang.ByteArray { return [0, 1, 127, 128, 255, -1, -128]b; }
    function sized(n as Lang.Number) as Lang.ByteArray { return new[n]b; }
    function computed(n as Lang.Number) as Lang.ByteArray { return [n, n + 1]b; }
}
function start() {
    var a = Bytes.literal();
    System.println(a.size());
    System.println(a.toString());
    System.println(a[5]);
    a[0] = -1;
    System.println(a[0]);
    var b = Bytes.sized(3);
    System.println(b.toString());
    System.println(b[0]);
    System.println(Bytes.computed(4).toString());
    var c = [1, 2]b;
    var shared = c;
    System.println(c.equals([1, 2]b));
    System.println(c.equals([1, 2]));
    System.println(c.add('A').equals(shared));
    System.println(shared.toString());
    c.addAll([3, 4]);
    c.addAll([5, 6]b);
    System.println(c.toString());
    System.println(c.indexOf('A'));
    System.println(c.remove(2));
    c.add(1);
    System.println(c.removeAll(1));
    System.println(c.toString());
    System.println(c.reverse().toString());
    System.println(c.slice(1, -1).toString());
    System.println(c.hashCode() == c.hashCode());
    System.println(c.method(:size).invoke());
    System.println(c.weak().stillAlive());
    System.exit();
}

class MinimalConsoleApp extends Toybox.Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { start(); }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
