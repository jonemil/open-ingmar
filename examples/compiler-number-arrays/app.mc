using Toybox.System;
module Numbers {
    function first(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number {
        return a[0];
    }
    function convert(text as Toybox.Lang.String) as Toybox.Lang.Array<Toybox.Lang.Number> {
        return text.toUtf8Array();
    }
    function effect() as Toybox.Lang.String {
        System.println("effect");
        return "z";
    }
}
function start() as Void {
    var a = "Aé😀".toUtf8Array();
    System.println(a.toString());
    System.println(a.size());
    System.println(a[1] + a[2]);
    System.println(Numbers.first(a));
    var shared = a;
    a.add(300);
    System.println(a[7]);
    System.println(a.remove(300));
    System.println(a.indexOf(195));
    System.println(a.reverse().toString());
    System.println(a.slice(1, 3).toString());
    var b = Numbers.convert("xy");
    a.addAll(b);
    System.println(shared.size());
    System.println(b.toString());
    System.println(a.removeAll(65));
    System.println(a[0]);
    System.println("".toUtf8Array().size());
    System.println(Numbers.effect().toUtf8Array()[0]);
    var text = "qr".substring(0, 1);
    if (!(text == null)) { System.println(text.toUtf8Array()[0]); }
    System.exit();
}
