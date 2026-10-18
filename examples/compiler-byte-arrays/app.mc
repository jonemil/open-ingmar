using Toybox.Lang;
using Toybox.System;
module Bytes {
    function make(n) as Lang.ByteArray { return new[n]b; }
    function echo(b as Lang.ByteArray) as Lang.ByteArray { return b; }
    function effect(n) { System.println(n); return n; }
    function choose(flag as Lang.Boolean) as Lang.ByteArray {
        return flag ? new[Bytes.effect(2)]b : new[Bytes.effect(4)]b;
    }
}
function start() {
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
    System.exit();
}
