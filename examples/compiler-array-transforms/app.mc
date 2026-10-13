using Toybox.System;

module ArrayOps {
    function count(chars as Toybox.Lang.Array<Toybox.Lang.Char>) as Toybox.Lang.Number {
        return chars.size();
    }
}

function start() {
    var chars = "abca".toCharArray();
    var reversed = chars.reverse();
    System.println(reversed.toString());
    System.println(reversed.indexOf('a'));
    System.println(ArrayOps.count(reversed));
    System.println(chars.toString());

    var sliced = chars.slice(1, 3);
    System.println(sliced.toString());
    System.println(sliced.indexOf('c'));
    System.println(ArrayOps.count(sliced));
    System.println(chars.slice(null, null).toString());
    System.println(chars.slice(-2, null).toString());
    System.println(chars.slice(-20, 20).toString());
    System.println(chars.slice(3, 1).toString());
    System.exit();
}
