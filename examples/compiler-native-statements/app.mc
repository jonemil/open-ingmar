using Toybox.System;

module Arrays {
    function mutate(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Void {
        a.add('c');
        a.addAll("da".toCharArray());
        a.remove('a');
        a.removeAll('a');
        a.size();
    }
}
module Effects {
    function receiver() as Toybox.Lang.Array<Toybox.Lang.Char> {
        System.println("receiver");
        return "aba".toCharArray();
    }
    function value() as Toybox.Lang.Char {
        System.println("value");
        return 'a';
    }
    function skipped() as Toybox.Lang.Boolean {
        System.println("unexpected");
        return false;
    }
}
function start() as Void {
    var a = "aba".toCharArray();
    var shared = a;
    Arrays.mutate(a);
    System.println(shared.toString());
    a.add('x').remove('x');
    System.println(a.toString());
    Effects.receiver().remove(Effects.value());
    Effects.receiver().remove((true || Effects.skipped()).hashCode().toChar());
    var i = 0;
    while (5 > i) {
        a.add('z');
        a.remove('z');
        i = i + 1;
    }
    System.println(a.toString());
    var text = "bc";
    text.find("z");
    text.substring(0, 1);
    text.toCharArray();
    7.abs();
    true.toString();
    'a'.toUpper();
    var maybe = text.substring(0, 1);
    if (!(maybe == null)) {
        maybe.toUpper();
        System.println(maybe);
    }
    System.println("done");
    System.exit();
}
