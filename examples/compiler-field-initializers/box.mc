using Toybox.Lang;
using Toybox.System;
class Token {
    var number = 41;
    function value() as Lang.Number { return self.number; }
}
class Box {
    var computed as Lang.Number = Trace.number(10);
    var number = 7;
    var snapshot as Lang.Number = self.number;
    var text = "field";
    var flag = true, letter = 'Q';
    var fraction = 1.5f;
    var big = -2147483649l;
    var wide = 2.5d;
    var optional as Lang.String or Null;
    var selected as Lang.String = true ? "yes" : "no";
    var items as Lang.Array<Lang.Number> = Trace.array();
    var values = {"value" => 4};
    var bytes as Lang.ByteArray = [1,2]b;
    var token = new Token();
    function initialize() { System.println(40); }
    function change() {
        self.text = "changed";
        self.items.add(3);
        self.token = new Token();
    }
}
