using Toybox.System;

module Arrays {
    function append(a as Toybox.Lang.Array<Toybox.Lang.Char>, b as Toybox.Lang.Array<Toybox.Lang.Char>) as Toybox.Lang.Array<Toybox.Lang.Char> {
        return a.addAll(b);
    }
    function receiver() as Toybox.Lang.Array<Toybox.Lang.Char> {
        System.println("receiver");
        return "aba".toCharArray();
    }
    function value() as Toybox.Lang.Char {
        System.println("value");
        return 'a';
    }
}

function start() as Void {
    var a = "aba".toCharArray();
    var shared = a;
    var b = "ca".toCharArray();
    var returned = Arrays.append(a, b);
    System.println(a.toString());
    System.println(shared.toString());
    System.println(b.toString());
    System.println(returned.remove('a'));
    System.println(a.toString());
    System.println(a.remove('z'));
    System.println(a.removeAll('a'));
    System.println(shared.toString());
    System.println(a.removeAll('a'));
    System.println(a.addAll("".toCharArray()).size());
    var empty = "".toCharArray();
    System.println(empty.remove('x'));
    System.println(empty.removeAll('x'));
    System.println(empty.addAll("xy".toCharArray()).toString());
    System.println(Arrays.receiver().remove(Arrays.value()));
    var guarded = "aba".toCharArray();
    System.println(false && guarded.removeAll('a'));
    System.println(guarded.toString());
    System.println(true && guarded.removeAll('a'));
    System.println(guarded.toString());
    var repeated = "xy".toCharArray();
    System.println("before self append");
    System.println(repeated.addAll(repeated).toString());
    System.exit();
}
