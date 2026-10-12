using Toybox.System;

function start() {
    var chars = "AbC".toCharArray();
    System.println(chars[0]);
    System.println(chars[1].toUpper());
    System.println(chars[2].toNumber());
    System.println(Arrays.first(chars));
    System.println(Arrays.effect()[Arrays.index()]);
    System.println("éΩ".toCharArray()[1]);
    System.println("😀".toCharArray()[0].toNumber());
    var i = 0;
    while (chars.size() > i) {
        System.println(chars[i]);
        i = i + 1;
    }
    var n = "1".toNumber();
    if (!(n == null) && chars[n] == 'b') { System.println("guarded"); }
    System.println(chars[(false || true).hashCode()]);
    System.exit();
}
