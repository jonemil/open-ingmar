using Toybox.System;

module Numbers {
    function showFind(text as Toybox.Lang.String, needle as Toybox.Lang.String) as Void {
        var index = text.find(needle);
        if (index == null) {
            System.println("not found");
        } else {
            System.println(index.format("%02d"));
        }
    }
    function parseOrZero(text as Toybox.Lang.String) as Toybox.Lang.Number {
        var number = text.toNumber();
        if (number == null) { return 0; }
        return number;
    }
}
