using Toybox.System;
module Compound {
    function value() as Toybox.Lang.Number { System.println("value"); return 3; }
    function flag() as Toybox.Lang.Boolean { System.println("flag"); return true; }
    function run() as Void {
        var n = 10;
        n += Compound.value(); System.println(n);
        n -= 2; System.println(n);
        n *= 3; System.println(n);
        n /= 4; System.println(n);
        n %= 3; System.println(n);
        n = 12; n &= 10; System.println(n);
        n |= 3; System.println(n);
        n ^= 15; System.println(n);
        n <<= 2; System.println(n);
        n >>= 1; System.println(n);
        n = -7; n %= 3; System.println(n);
        n = -8; n >>= 1; System.println(n);
        n = 1; n <<= 31; System.println(n);
        n = 10; n += n; System.println(n);
        n += (false || Compound.flag()).hashCode(); System.println(n);
        var values = [0, 0];
        var index = 0;
        while (index < values.size()) {
            values[index] = index * 10;
            index += 1;
        }
        System.println(values.toString());
        var converted = "41".toNumber();
        if (converted != null) {
            converted += 1;
            if (converted != null) { System.println(converted); }
        }
        var text = "n=";
        text += n; text += 'A'; System.println(text);
        var character = 'A';
        character += 2; character += -1; System.println(character);
        System.println("done");
    }
}
