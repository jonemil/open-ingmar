using Toybox.System;
import Toybox.Lang;
using Toybox.WatchUi;
module SourceTests {
    function read(o as Toybox.Lang.Object or Null) as Toybox.Lang.Number {
        if (o instanceof Probe) { return o.value(); } else { return -1; }
    }
    function asserted(o as Toybox.Lang.Object) as Probe { return o as Probe; }
    function view(o as Toybox.Lang.Object) as Toybox.Lang.Number {
        if (o instanceof LocalView) { return o.value(); } else { return -1; }
    }
    function run() as Void {
        var p = new Probe();
        System.println((p as Lang.Object) instanceof Probe);
        System.println(SourceTests.read(p));
        System.println(SourceTests.read(new NumberOrder()));
        System.println(SourceTests.read(null));
        var erased = p as Lang.Object;
        System.println(SourceTests.asserted(erased).value());
        System.println((erased as Probe).equals(p));
        System.println((3 as Probe) instanceof Lang.Number);
        System.println((null as Probe) instanceof Probe);
        var object = erased;
        if (object instanceof Probe) { System.println(object.value()); object = new NumberOrder(); }
        System.println(object instanceof Probe);
        var reference = erased;
        while (reference instanceof Probe) { System.println(reference.value()); reference = new NumberOrder(); }
        System.println(SourceTests.view(new LocalView()));
        var delegate = new LocalDelegate() as Lang.Object;
        System.println((delegate as LocalDelegate).value());
    }
}
class LocalView extends WatchUi.View {
    function initialize() { View.initialize(); }
    function value() { return 2; }
}
class LocalDelegate extends WatchUi.BehaviorDelegate {
    function initialize() { BehaviorDelegate.initialize(); }
    function value() { return 3; }
}

module NullableSourceTests {
    function pass(token as Probe or Null) as Probe or Null { return token; }
    function choose(token as Probe or Null) as Probe or Null {
        if (token == null) { return new Probe(); }
        return null;
    }
    function read(token as Probe or Null) as Number {
        var copy = NullableSourceTests.pass(token);
        if (copy != null) { return copy.value(); }
        return -1;
    }
    function view(token as LocalView or Null) as Number {
        if (token != null) { return token.value(); }
        return -2;
    }
    function delegate(token as LocalDelegate or Null) as Number {
        if (token != null) { return token.value(); }
        return -3;
    }
    function run() as Void {
        System.println(NullableSourceTests.read(new Probe()));
        System.println(NullableSourceTests.read(null));
        System.println(NullableSourceTests.view(new LocalView()));
        System.println(NullableSourceTests.view(null));
        System.println(NullableSourceTests.delegate(new LocalDelegate()));
        System.println(NullableSourceTests.delegate(null));
        var token = NullableSourceTests.choose(null);
        var copy = token;
        if (copy != null) { System.println(copy.value()); }
        copy = null;
        System.println(copy == null);
        token = new Probe();
        while (token != null) { System.println(token.value()); token = null; }
        System.println(NullableSourceTests.choose(new Probe()) == null);
        var holder = new NullableSourceHolder();
        holder.run();
        NullableSourceCasts.run();
        SourceJoins.run();
        CoreLoops.run();
        CoreScopes.run();
        CoreHeaders.run();
        CoreNonreturning.run();
        CoreDeclarations.run();
        CoreSwitch.run();
        CoreLiterals.run();
        CoreEscapes.run();
        CoreLists.run();
    }
}

module ImportedTypes {
    function number(value as Object) as Number { return value as Number + 1; }
    function array(values as Array<String>) as Number { return values.size(); }
    function dictionary(values as Dictionary<String, Number>) as Number { return values.size(); }
    function nullable(value as Object or Null) as String or Null { return value as String or Null; }
    function isChar(value as Object) as Boolean { return value instanceof Char; }
    function run() as Void {
        System.println(ImportedTypes.number(7));
        System.println(ImportedTypes.array(["a", "b"]));
        System.println(ImportedTypes.dictionary({"a" => 1}));
        System.println(ImportedTypes.isChar('a'));
        System.println(ImportedTypes.nullable(null) == null);
        var object = "hello" as Object;
        System.println((object as String).length());
        System.println(object instanceof String);
    }
}

class NullableSourceHolder {
    var token as Probe or Null;
    var view as LocalView or Null = null;
    var delegate as LocalDelegate or Null;
    function initialize() {}
    function load() as Probe or Null { return self.token; }
    function snapshot(token as Probe) {
        self.token = token;
        var copy = self.load();
        self.token = null;
        if (copy != null) { return copy.value(); }
        return -1;
    }
    function run() as Void {
        System.println(self.token == null);
        System.println(self.view == null);
        System.println(self.delegate == null);
        self.token = new Probe();
        self.view = new LocalView();
        self.delegate = new LocalDelegate();
        var token = self.token;
        var view = self.view;
        var delegate = self.delegate;
        self.token = null;
        self.view = null;
        self.delegate = null;
        if (token != null) { System.println(token.value()); }
        if (view != null) { System.println(view.value()); }
        if (delegate != null) { System.println(delegate.value()); }
        System.println(self.snapshot(new Probe()));
        var again = self.load();
        System.println(again == null);
    }
}

module NullableSourceCasts {
    function effect() as Object or Null {
        System.println("source-cast-effect");
        return new Probe();
    }
    function cast(value as Object or Null) as Probe or Null {
        return value as Probe or Null;
    }
    function run() as Void {
        var token = NullableSourceCasts.cast(new Probe());
        if (token != null) { System.println(token.value()); }
        token = NullableSourceCasts.cast(null);
        System.println(token == null);
        var direct = null as Probe or Null;
        System.println(direct == null);
        var view = new LocalView() as Object;
        var recovered = view as LocalView or Null;
        if (recovered != null) { System.println(recovered.value()); }
        var delegate = new LocalDelegate() as Object;
        var local = delegate as LocalDelegate or Null;
        if (local != null) { System.println(local.value()); }
        var effect = NullableSourceCasts.effect() as Probe or Null;
        if (effect != null) { System.println(effect.value()); }
        System.println((3 as Probe or Null) instanceof Number);
        System.println((new LocalView() as Probe or Null) instanceof LocalView);
        System.println(NullableSourceCasts.cast(null) == null or false);
        var asserted = (new Probe() as Probe or Null) as Probe;
        System.println(asserted.value());
    }
}

module SourceJoins {
    function choose(flag as Boolean) as Probe or Null {
        return flag ? new Probe() : null;
    }
    function fromObject(value as Object) as Probe or Null {
        return value instanceof Probe ? value : null;
    }
    function left() as Probe { System.println("source-join-left"); return new Probe(); }
    function right() as Probe { System.println("source-join-right"); return new Probe(); }
    function run() as Void {
        var same = true ? new Probe() : new Probe();
        System.println(same.value());
        var maybe = SourceJoins.choose(false);
        System.println(maybe == null);
        maybe = SourceJoins.choose(true);
        if (maybe != null) { System.println(maybe.value()); }
        var nested = false ? null : (true ? new Probe() : null);
        if (nested != null) { System.println(nested.value()); }
        var view = true ? new LocalView() : null;
        if (view != null) { System.println(view.value()); }
        var delegate = false ? null : new LocalDelegate();
        if (delegate != null) { System.println(delegate.value()); }
        var first = true ? SourceJoins.left() : SourceJoins.right();
        System.println(first.value());
        var second = false ? SourceJoins.left() : SourceJoins.right();
        System.println(second.value());
        var guarded = SourceJoins.fromObject(new Probe());
        if (guarded != null) { System.println(guarded.value()); }
        System.println(SourceJoins.fromObject(new NumberOrder()) == null);
    }
}

module CoreLoops {
    function begin() { System.println("loop-init"); return 0; }
    function test(n as Number) as Boolean { System.println("loop-test"); return n < 2; }
    function step(n as Number) { System.println("loop-step"); return n + 1; }
    function falseMark() as Boolean { System.println("bad-do-test"); return false; }
    function returnDo() { do { return 41; } while(CoreLoops.falseMark()); }
    function returnFor() { for(;;) { return 42; } }
    function proof(value as Probe or Null) as Void {
        if (value != null) {
            do { break; } while(CoreLoops.falseMark());
            System.println(value.value());
        }
    }
    function run() as Void {
        var sum = 0;
        for (var i = 0; i < 6; i++) {
            if (i == 1) { continue; }
            if (i == 5) { break; }
            sum += i;
        }
        System.println(sum);
        var n = 0;
        do { n++; if (n < 3) { continue; } break; } while(true);
        System.println(n);
        var once = 0;
        do { once++; } while(false);
        System.println(once);
        for (var trace = CoreLoops.begin(); CoreLoops.test(trace); trace = CoreLoops.step(trace)) {
            System.println(trace);
        }
        for (var j = 0; j < 2; j = CoreLoops.step(j)) { break; }
        var j = 7;
        System.println(j);
        var k = 0;
        for (; k < 2;) { k++; }
        System.println(k);
        for (;;) { break; }
        System.println("for-empty");
        var count = 0;
        for (var outer = 0; outer < 3; outer++) {
            var inner = 0;
            do { inner++; if (inner == 2) { continue; } count++; } while(inner < 3);
        }
        System.println(count);
        System.println(CoreLoops.returnDo());
        System.println(CoreLoops.returnFor());
        var nullable = "8".toNumber();
        do {
            if (nullable == null) { break; }
            System.println(nullable);
            nullable = null;
        } while(nullable != null);
        CoreLoops.proof(new Probe());
    }
}

module CoreScopes {
    function test(value as Number, expected as Number) as Boolean {
        System.println("scope-test");
        System.println(expected);
        return value == expected;
    }
    function choose(value as Number) as Number {
        {
            if (CoreScopes.test(value, 0)) { { return 41; } }
            else if (CoreScopes.test(value, 1)) { return 42; }
            else { return 43; }
        }
    }
    function guarded(value as Number or Null) as Void {
        {
            if (value == null) { System.println("scope-null"); }
            else if (value > 0) { System.println(value + 1); }
            else { System.println(value - 1); }
        }
    }
    function run() as Void {
        var sum = 0;
        { var step = 1; sum += step; }
        { var step = 2; sum += step; }
        System.println(sum);
        var step = 4;
        sum += step;
        System.println(sum);
        System.println(CoreScopes.choose(0));
        System.println(CoreScopes.choose(1));
        System.println(CoreScopes.choose(2));
        var total = 0;
        {
            for (var i = 0; i < 4; i++) {
                {
                    if (i == 1) { continue; }
                    else if (i == 3) { break; }
                    else { total += i; }
                }
            }
        }
        System.println(total);
        CoreScopes.guarded(null);
        CoreScopes.guarded(2);
        CoreScopes.guarded(0);
    }
}

module CoreHeaders {
    function mark(label as String, value as Number) as Number {
        System.println(label);
        System.println(value);
        return value;
    }
    function index() as Number {
        System.println("header-index");
        return 0;
    }
    function run() as Void {
        for (var i = CoreHeaders.mark("header-init-i", 0),
                 j = CoreHeaders.mark("header-init-j", i + 1);
             i < 3;
             i = CoreHeaders.mark("header-step-i", i + 1),
             j = CoreHeaders.mark("header-step-j", j + i)) {
            if (i == 1) { continue; }
            System.println(j);
        }
        var i = 0;
        var j = 0;
        for (i = CoreHeaders.mark("header-assign-i", 1),
             j = CoreHeaders.mark("header-assign-j", i + 1);
             i < 3; i++, j += i) {
            System.println(j);
        }
        for (i = 0, j = 0; i < 2;
             i = CoreHeaders.mark("bad-header-update", i + 1), j += i) {
            break;
        }
        System.println(j);
        var values = [0];
        for (i = 0; i < 2; values[CoreHeaders.index()] += i, i++) {}
        System.println(values[0]);
    }
}

module CoreNonreturning {
    function number() as Number { for (;;) {} }
    function string() as String { for (;;) {} }
    function source() as Probe { for (;;) {} }
    function chooseNumber(flag as Boolean) as Number {
        if (flag) { return 41; }
        else { return CoreNonreturning.number(); }
    }
    function chooseString(flag as Boolean) as String {
        if (flag) { return "nonreturning-types"; }
        else { return CoreNonreturning.string(); }
    }
    function chooseSource(flag as Boolean) as Probe {
        if (flag) { return new Probe(); }
        else { return CoreNonreturning.source(); }
    }
    function run() as Void {
        System.println(CoreNonreturning.chooseNumber(true));
        System.println(CoreNonreturning.chooseString(true));
        var value = CoreNonreturning.chooseSource(true);
        System.println(value.value());
    }
}

module CoreDeclarations {
    function run() as Void {
        var first = CoreHeaders.mark("local-first", 1),
            second = CoreHeaders.mark("local-second", first + 2),
            text = "locals";;;
        System.println(second);;;
        System.println(text);;;
        var number = 0, deltas = [1, 2];;
        number += deltas[1];;
        number++;;;
        System.println(number);;;
        do { number++;;; } while (false);;;
        System.println(number);
        { var inner = 5, copy = inner + 1; System.println(copy); };;;
        var inner = 7, copy = inner + 1;
        System.println(copy);
        var guarded = "8".toNumber(), same = guarded;
        if (same != null) { System.println(same + 1); }
    }
}

module CoreSwitch {
    function custom() as Number {
        switch (new SwitchToken()) { case 0: return 51; default: return 52; }
    }
    function numeric(value as Number) as Number {
        switch (value) {
            case 0: System.println("switch-zero");
            case 1: return 11;
            default: return 19;
        }
    }
    function defaultFirst(value as Number) as Number {
        switch (value) { default: return 29; case 0: return 20; }
    }
    function middle(value as Number) as Number {
        var n = 0;
        switch (value) {
            case 0: n += 1;
            default: n += 3;
            case 1: n += 2;
        }
        return n;
    }
    function dynamic() as Number {
        switch (CoreHeaders.mark("switch-selector", 2)) {
            default: return 30;
            case CoreHeaders.mark("switch-case-a", 1): return 31;
            case CoreHeaders.mark("switch-case-b", 2): return 32;
            case CoreHeaders.mark("bad-switch-case", 3): return 33;
        }
    }
    function tag(value as Object) as Number {
        switch (value) {
            case instanceof Number: return (value as Number) + 1;
            case instanceof String: return (value as String).length();
            case instanceof Probe: return (value as Probe).value();
            default: return 0;
        }
    }
    function nullable(value as Number or Null) as Number {
        if (value == null) { return 0; }
        switch (value) { case 4: return 5; default: return value + 1; }
    }
    function word(value as String) as Number {
        switch (value) { case "ab": return 1; default: return 0; }
    }
    function loops() as Number {
        var sum = 0;
        for (var i = 0; i < 4; i++) {
            switch (i) { case 1: continue; case 3: break; default: sum += i; }
            sum += 10;
        }
        return sum;
    }
    function scopes(value as Number) as Number {
        switch (value) {
            case 0: { var n = 40; return n; }
            default: { var n = 41; return n; }
        }
    }
    function run() as Void {
        System.println(CoreSwitch.numeric(0));
        System.println(CoreSwitch.numeric(1));
        System.println(CoreSwitch.numeric(2));
        System.println(CoreSwitch.defaultFirst(0));
        System.println(CoreSwitch.defaultFirst(2));
        System.println(CoreSwitch.middle(0));
        System.println(CoreSwitch.middle(1));
        System.println(CoreSwitch.middle(2));
        System.println(CoreSwitch.dynamic());
        System.println(CoreSwitch.tag(4));
        System.println(CoreSwitch.tag("abc"));
        System.println(CoreSwitch.tag(new Probe()));
        System.println(CoreSwitch.tag(true));
        System.println(CoreSwitch.nullable(null));
        System.println(CoreSwitch.nullable(4));
        System.println(CoreSwitch.word("ab"));
        var prefix = "a";
        System.println(CoreSwitch.word(prefix + "b"));
        System.println(CoreSwitch.loops());
        System.println(CoreSwitch.scopes(0));
        System.println(CoreSwitch.scopes(1));
        System.println(CoreSwitch.custom());
    }
}

class SwitchToken {
    function initialize() {}
    function equals(other as Object or Null) as Boolean {
        System.println("switch-equals");
        return true;
    }
}

module CoreLiterals {
    function run() as Void {
        System.println("numeric-literals");
        System.println(0xFF);
        System.println(0xffffffff);
        System.println(-0xffffffff);
        System.println(077);
        System.println(037777777777);
        System.println(-037777777777);
        System.println(0xffffffffL);
        System.println(0xffffffffffffffffL);
        System.println(-0xffffffffffffffffL);
        System.println(077L);
        System.println(1.25f == 1.25);
        System.println(.5 == 0.5);
        System.println(.5d == 0.5d);
        System.println(1f instanceof Float);
        System.println(1d instanceof Double);
        System.println(NaN instanceof Float);
        System.println(NaN instanceof Double);
        var nan = NaN;
        System.println(nan == nan);
        System.println(NaN != NaN);
        var floats = [NaN, 1f];
        System.println(floats[0] instanceof Float);
        System.println(floats[0] == floats[0]);
        var words = [0xffffffffffffffffL, 077L];
        System.println(words[0]);
        System.println(words[1]);
        var holder = new LiteralHolder();
        System.println(holder.value());
    }
}

class LiteralHolder {
    var marker = 0xffffffff;
    function initialize() {}
    function value() as Number { return self.marker; }
}

module CoreEscapes {
    function run() as Void {
        System.println("source-escapes");
        System.println('\b'.toNumber());
        System.println('\f'.toNumber());
        System.println('\"'.toNumber());
        System.println('\u0000'.toNumber());
        System.println('\u00e9'.toNumber());
        System.println('\u732B'.toNumber());
        System.println('\uD83D'.toNumber());
        System.println('\ude00'.toNumber());
        System.println('\uFFFF'.toNumber());
        System.println("\u0041\u00e9\u732B");
        System.println("\uD83D\uDE00".equals("😀"));
        System.println("\b\f".equals("\u0008\u000C"));
        System.println("\'\"\\".equals("\u0027\u0022\u005c"));
        System.println("a\u0000b".length());
        System.println("\\u0041");
        var chars = ['\u0041', '\uD83D', '\ude00'];
        System.println(chars[0].toNumber());
        System.println(chars[1].toNumber());
        System.println(chars[2].toNumber());
    }
}

module CoreLists {
    function show(value as Number or Null) as Void {
        if (value != null) { System.println(value); } else { System.println(-1); }
    }
    function run() as Void {
        System.println("container-commas");
        var numbers = [1, 2, /* last */ ];
        System.println(numbers.size());
        System.println(numbers[1]);
        var chars = ['a', 'b',];
        System.println(chars[1].toNumber());
        var bytes = [1, 255,]b;
        System.println(bytes.size());
        System.println(bytes[1]);
        var words = {"one" => 1, "two" => 2,};
        System.println(words.size());
        CoreLists.show(words["two"]);
        var computed = [CoreHeaders.mark("array-first", 3), CoreHeaders.mark("array-last", 4),];
        System.println(computed[1]);
        var computedBytes = [CoreHeaders.mark("byte-first", 5), CoreHeaders.mark("byte-last", 6),]b;
        System.println(computedBytes[1]);
        var computedWords = {"one" => CoreHeaders.mark("dictionary-first", 7), "two" => CoreHeaders.mark("dictionary-last", 8),};
        CoreLists.show(computedWords["two"]);
        var branching = [true ? CoreHeaders.mark("array-branch", 9) : 0,];
        System.println(branching[0]);
    }
}
