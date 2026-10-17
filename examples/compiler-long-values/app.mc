using Toybox.Application;
using Toybox.System;

module Wide {
    function parse(text as Toybox.Lang.String) as Toybox.Lang.Long or Null { return text.toLong(); }
    function nullable(value as Toybox.Lang.Long or Null) as Toybox.Lang.Long or Null { return value; }
    function chooseLong(present as Toybox.Lang.Boolean) as Toybox.Lang.Long or Null {
        return present ? 4294967297l : null;
    }
    function show(value as Toybox.Lang.Long or Null) as Void {
        if (value != null) { System.println(value); } else { System.println("null"); }
    }
    function textEffect(value as Toybox.Lang.String) as Toybox.Lang.String {
        System.println("text"); return value;
    }
    function methods() as Void {
        var value = Wide.pass(-4294967297l);
        System.println(value.abs());
        System.println(Wide.zero().abs());
        System.println(value.toLong());
        System.println(value.toNumber());
        System.println(Wide.pass(4294967295l).toNumber());
        System.println(Wide.pass(4294967296l).toNumber());
        System.println(Wide.maximum().toNumber());
        System.println(Wide.minimum().toNumber());
        System.println(Wide.numberEffect(-1).toLong());
        System.println(Wide.numberEffect(-2147483648).toLong());
        System.println(Wide.numberEffect(2147483647).toLong());
        System.println(Wide.maximum().toString());
        System.println(value.format("%d"));
        System.println(Wide.maximum().format("%016x"));
        System.println(Wide.pass(42l).format("%+d"));
        System.println(Wide.pass(1l).equals(1l));
        System.println(Wide.pass(1l).equals(1));
        System.println(1.equals(1l));
        System.println(Wide.pass(1l).equals(true));
        System.println(Wide.pass(65l).equals('A'));
        System.println(Wide.pass(1l).equals("1"));
        System.println("1".equals(1l));
        System.println(false.equals(0l));
        System.println('A'.equals(65l));
        System.println({}.equals(1l));
        System.println(value.equals(null));
        System.println(value.hashCode() == Wide.pass(-4294967297l).hashCode());
        System.println(value.hashCode() == value.hashCode());
        System.println(value.weak().equals(value));
        System.println(Wide.maximum().weak().toString());
        System.println(Wide.pass(3l).compareTo(2l) > 0);
        System.println(Wide.pass(3l).compareTo(3) == 0);
        System.println(Wide.pass(1l).compareTo(true) == 0);
        System.println(Wide.pass(65l).compareTo('A') == 0);
        System.println(Wide.maximum().compareTo(Wide.minimum()) > 0);
        System.println(Wide.minimum().compareTo(Wide.maximum()) < 0);
        System.println(3.compareTo(3l) == 0);
        System.println(true.compareTo(1l) == 0);
        System.println('A'.compareTo(65l) == 0);
        System.println(Wide.longEffect(42l).format(Wide.textEffect("%d")));
        System.println(Wide.longEffect(3l).compareTo(Wide.numberEffect(2)) > 0);
        System.println(Wide.longEffect(3l).equals(Wide.longEffect(3l)));
        Wide.longEffect(-5l).abs();
        Wide.longEffect(5l).toLong();
        Wide.longEffect(5l).hashCode();
        Wide.show(Wide.parse("4294967297"));
        Wide.show(Wide.parse("-9223372036854775808"));
        Wide.show(Wide.parse("9223372036854775807"));
        Wide.show(Wide.parse("9223372036854775808"));
        Wide.show(Wide.parse("-9223372036854775809"));
        Wide.show(Wide.parse("bad"));
        Wide.show(Wide.parse("42tail"));
        Wide.show(Wide.parse("3.14"));
        Wide.show("100000000".toLongWithBase(16));
        Wide.show("0x100000000".toLongWithBase(0));
        Wide.show("101".toLongWithBase(2));
        Wide.show("z".toLongWithBase(36));
        Wide.show("bad".toLongWithBase(2));
        Wide.show(Wide.nullable(null));
        Wide.show(Wide.nullable(42l));
        var parsed = Wide.chooseLong(false);
        while (parsed == null) { parsed = Wide.parse("4294967297"); }
        System.println(parsed.abs());
        var copy = Wide.nullable(parsed);
        parsed = null;
        Wide.show(copy);
        System.println(value.equals(parsed));
        parsed = 7l;
        if (parsed != null) { System.println(parsed.weak().toString()); }
        var text = "4294967297".substring(0, 10);
        if (text != null) { Wide.show(text.toLong()); }
        System.println(Wide.minimum().abs());
    }
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
        Wide.methods();
        System.exit();
    }
}
