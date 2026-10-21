using Toybox.System;

function start() as Void {
    var chars = "AbA".toCharArray();
    System.println(chars.size());
    System.println(chars.toString());
    System.println(chars.indexOf('A'));
    System.println(chars.indexOf('b'));
    System.println(chars.indexOf('z'));
    System.println("".toCharArray().size());
    System.println("".toCharArray().toString());
    System.println("éΩ".toCharArray().size());
    System.println("éΩ".toCharArray().indexOf('Ω'));
    System.println("😀".toCharArray().size());
    System.println("😀".toCharArray().indexOf(128512.toChar()));
    System.println("😀".toCharArray().indexOf(55357.toChar()));
    System.println(Arrays.count(chars));
    chars = Arrays.convert("xy");
    System.println(chars.toString());
    var text = "abc".substring(1, 3);
    if (!(text == null)) { System.println(text.toCharArray().toString()); }
    System.println(Arrays.effect().toCharArray().size());
    System.println(chars.size() > 0 && chars.indexOf('x') == 0);
    System.exit();
}
