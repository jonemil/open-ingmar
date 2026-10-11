module Values {
    function parse(text as Toybox.Lang.String) as Toybox.Lang.Number or Null {
        return text.toNumber();
    }

    function numberOrZero(value as Toybox.Lang.Number or Null)
            as Toybox.Lang.Number {
        var number = value;
        if (number == null) { return 0; }
        return number;
    }

    function textOrFallback(value as Toybox.Lang.String or Null)
            as Toybox.Lang.String {
        var text = value;
        if (text == null) { return "fallback"; }
        return text;
    }
}
