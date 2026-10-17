using Toybox.Application;
using Toybox.System;

module Wide {
    function shift(value as Toybox.Lang.Long, count as Toybox.Lang.Number) as Toybox.Lang.Long {
        return value << count;
    }
    function shiftNumber(value as Toybox.Lang.Number, count as Toybox.Lang.Long) as Toybox.Lang.Long {
        return value << count;
    }
    function right(value as Toybox.Lang.Long, count as Toybox.Lang.Number) as Toybox.Lang.Long {
        return value >> count;
    }
    function bits() as Void {
        var value = Wide.pass(4294967297l);
        System.println(value & 3);
        System.println(3 & value);
        System.println(value | 2);
        System.println(2 | value);
        System.println(value ^ 1);
        System.println(1 ^ value);
        System.println(value & Wide.pass(4294967296l));
        System.println(value | Wide.pass(4294967296l));
        System.println(value ^ Wide.pass(4294967296l));
        System.println(~value);
        System.println(!value);
        System.println(~Wide.zero());
        System.println(!Wide.zero());
        System.println(~Wide.minimum());
        System.println(!Wide.maximum());
        System.println(Wide.shift(1l, 0));
        System.println(Wide.shift(1l, 31));
        System.println(Wide.shift(1l, 32));
        System.println(Wide.shift(1l, 63));
        System.println(Wide.shift(1l, 64));
        System.println(Wide.shift(1l, 65));
        System.println(Wide.shiftNumber(1, 32l));
        System.println(value >> 32);
        System.println(1 >> Wide.pass(32l));
        System.println(value << Wide.pass(1l));
        System.println(value >> Wide.pass(1l));
        System.println(Wide.right(-8l, 1));
        System.println(Wide.right(Wide.minimum(), 63));
        System.println(Wide.right(Wide.minimum(), 64));
        System.println(Wide.right(Wide.minimum(), 65));
        System.println(Wide.longEffect(1l) << Wide.numberEffect(32));
        System.println(Wide.numberEffect(1) << Wide.longEffect(32l));
        var updated = Wide.maximum();
        updated++; System.println(updated);
        --updated; System.println(updated);
        updated = Wide.minimum();
        updated--; System.println(updated);
        ++updated; System.println(updated);
        updated = Wide.pass(4294967297l);
        updated &= 3; System.println(updated);
        updated |= 4294967296l; System.println(updated);
        updated ^= 1; System.println(updated);
        updated <<= 1; System.println(updated);
        updated >>= 32l; System.println(updated);
        System.println(Wide.shift(1l, -1));
        System.println(Wide.shift(1l, -64));
        System.println(Wide.shift(1l, -65));
        System.println(Wide.shiftNumber(1, 4294967296l));
        System.println(Wide.shiftNumber(-1, -1l));
        System.println(Wide.right(-8l, -1));
    }
    function longEffect(value as Toybox.Lang.Long) as Toybox.Lang.Long {
        System.println("long"); return value;
    }
    function numberEffect(value as Toybox.Lang.Number) as Toybox.Lang.Number {
        System.println("number"); return value;
    }
    function add(left as Toybox.Lang.Long, right as Toybox.Lang.Number) as Toybox.Lang.Long {
        return left + right;
    }
    function subtract(left as Toybox.Lang.Number, right as Toybox.Lang.Long) as Toybox.Lang.Long {
        return left - right;
    }
    function negate(value as Toybox.Lang.Long) as Toybox.Lang.Long { return -value; }
    function positive(value as Toybox.Lang.Long) as Toybox.Lang.Long { return +value; }
    function arithmetic() as Void {
        var value = Wide.pass(4294967297l);
        var number = 3;
        System.println(value + number);
        System.println(number + value);
        System.println(value - number);
        System.println(number - value);
        System.println(value * number);
        System.println(number * value);
        System.println(value / number);
        System.println(number / value);
        System.println(value % number);
        System.println(number % value);
        System.println(Wide.pass(-7l) / 3);
        System.println(Wide.pass(-7l) % 3);
        System.println(-7 / Wide.pass(3l));
        System.println(-7 % Wide.pass(3l));
        System.println(Wide.pass(-7l) / Wide.pass(3l));
        System.println(Wide.pass(-7l) % Wide.pass(3l));
        System.println(Wide.pass(7l) / Wide.pass(-3l));
        System.println(Wide.pass(7l) % Wide.pass(-3l));
        System.println(Wide.add(value, number));
        System.println(Wide.subtract(number, value));
        System.println(Wide.negate(value));
        System.println(Wide.positive(value));
        System.println(Wide.pass(1l) == 1);
        System.println(Wide.pass(1l) != 2);
        System.println(Wide.pass(2l) > 1);
        System.println(Wide.pass(1l) < 2);
        System.println(1 <= Wide.pass(1l));
        System.println(1 >= Wide.pass(1l));
        System.println(-1 < Wide.pass(4294967296l));
        System.println(value == Wide.pass(4294967297l));
        System.println(value != Wide.pass(4294967297l));
        System.println(value < Wide.pass(4294967298l));
        System.println(value > Wide.pass(4294967298l));
        System.println(value <= Wide.pass(4294967297l));
        System.println(value >= Wide.pass(4294967297l));
        System.println(Wide.longEffect(10l) + Wide.numberEffect(3));
        System.println(Wide.numberEffect(3) - Wide.longEffect(10l));
        System.println(Wide.longEffect(10l) +
            (true ? Wide.numberEffect(3) : Wide.numberEffect(4)));
        var updated = 10l;
        updated += 3; System.println(updated);
        updated -= 2l; System.println(updated);
        updated *= 3; System.println(updated);
        updated /= 2; System.println(updated);
        updated %= 3; System.println(updated);
        var maximum = Wide.maximum();
        var minimum = Wide.minimum();
        System.println(maximum + 1l);
        System.println(minimum - 1l);
        System.println(maximum + 1);
        System.println(minimum - 1);
        System.println(Wide.pass(4611686018427387904l) * 4l);
        System.println(-minimum);
    }
    function pass(value as Toybox.Lang.Long) as Toybox.Lang.Long { return value; }
    function zero() as Toybox.Lang.Long { return 0l; }
    function minimum() as Toybox.Lang.Long { return -9223372036854775808l; }
    function maximum() as Toybox.Lang.Long { return 9223372036854775807L; }
}

class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function pass(value as Toybox.Lang.Long) as Toybox.Lang.Long { return value; }
    function onStart(state) {
        System.println(0l);
        System.println(1l);
        System.println(-1l);
        System.println(2147483648l);
        System.println(4294967296l);
        System.println(72623859790382856l);
        System.println(Wide.minimum());
        System.println(Wide.maximum());
        var value = Wide.pass(4294967297l);
        var copied = value;
        value = -4294967297l;
        System.println(copied);
        System.println(self.pass(value));
        System.println(Wide.pass(Wide.zero()));
        Wide.pass(42l);
        var n = 0;
        while (n < 2) { System.println(Wide.pass(256l)); n++; }
        Wide.arithmetic();
        Wide.bits();
        System.exit();
    }
}
