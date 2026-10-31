using Toybox.Lang;
using Toybox.System;
class Box {
    var VALUE as Lang.Number=7;
    var snapshot as Lang.Number=VALUE;
    var LABEL as Lang.String="field";
    var CAPTURE=REF;
    var REF="later";
    var M as Lang.String="shadow";
    var ITEMS as Lang.Array<Lang.Number> = [1,2];
    function read() as Lang.Number { return VALUE; }
    function label() as Lang.String { return LABEL; }
    function change() { VALUE+=2; VALUE++; --VALUE; ITEMS[0]+=1; ITEMS.add(VALUE); }
    function parameter(VALUE as Lang.Number) as Lang.Number { VALUE++; return VALUE+self.VALUE; }
    function local() as Lang.Number { var VALUE=20; VALUE+=1; return VALUE+self.VALUE; }
    function qualifier() as Lang.Number { return M.length(); }
    function globalModule() as Lang.Number { return $.M.length(); }
    function forward() { System.println(VALUE); var VALUE=30; System.println(VALUE); }
    function scope() { if (true) { var VALUE=40; System.println(VALUE); } System.println(VALUE); }
}
