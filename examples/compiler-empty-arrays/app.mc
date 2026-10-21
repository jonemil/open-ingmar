using Toybox.System;
module Empty {
    function numbers() as Toybox.Lang.Array<Toybox.Lang.Number> {
        return [];
    }
    function chars() as Toybox.Lang.Array<Toybox.Lang.Char> {
        return ([]);
    }
    function count(a as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Number {
        return a.size();
    }
    function show(a as Toybox.Lang.Array<Toybox.Lang.Char>) as Void {
        System.println(a.size());
    }
    function effect(n as Toybox.Lang.Number) as Toybox.Lang.Number {
        System.println(n);
        return n;
    }
    function ordered(n as Toybox.Lang.Number, a as Toybox.Lang.Array<Toybox.Lang.Number>, m as Toybox.Lang.Number) as Toybox.Lang.Number {
        return n + a.size() + m;
    }
    function sized(n as Toybox.Lang.Number) as Toybox.Lang.Array<Toybox.Lang.Number or Null> {
        return new[n];
    }
    function nullableCount(a as Toybox.Lang.Array<Toybox.Lang.Number or Null>) as Toybox.Lang.Number {
        return a.size();
    }
    function select(flag as Toybox.Lang.Boolean,
                    a as Toybox.Lang.Array<Toybox.Lang.Number or Null>,
                    b as Toybox.Lang.Array<Toybox.Lang.Number or Null>)
                    as Toybox.Lang.Array<Toybox.Lang.Number or Null> {
        return flag ? a : b;
    }
    function chooseSized(flag as Toybox.Lang.Boolean)
                        as Toybox.Lang.Array<Toybox.Lang.Number or Null> {
        return flag ? new[Empty.effect(2)] : new[Empty.effect(4)];
    }
}
function start() as Void {
    var a = Empty.numbers();
    System.println(a.size());
    a.add(7);
    System.println(a.toString());
    var shared = a;
    a = [];
    System.println(a.size());
    System.println(shared.toString());
    a.addAll([]);
    System.println(a.size());
    System.println(Empty.count([]));
    Empty.show([]);
    var chars = Empty.chars();
    chars.add('x');
    chars.addAll([]);
    System.println(chars.toString());
    chars = [];
    System.println(chars.size());
    System.println(Empty.ordered(Empty.effect(10), [], Empty.effect(20)));
    var sized = Empty.sized(Empty.effect(3));
    System.println(sized.toString());
    System.println(sized[0] == null);
    sized[1] = 7;
    var old = sized;
    sized = new[2];
    System.println(sized.toString());
    System.println(old.toString());
    System.println(Empty.nullableCount(new[Empty.effect(0)]));
    sized = new[true ? Empty.effect(1) : Empty.effect(9)];
    System.println(sized.size());
    System.println(sized[0] == null);
    var selected = Empty.select(true, sized, old);
    selected[0] = 9;
    System.println(sized.toString());
    System.println(old.toString());
    selected = Empty.select(false, sized, old);
    selected[0] = 8;
    System.println(old.toString());
    System.println(Empty.chooseSized(true).toString());
    System.println(Empty.chooseSized(false).toString());
    System.exit();
}
