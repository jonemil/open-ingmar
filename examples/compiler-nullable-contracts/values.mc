module Values {
    function parse(text as Toybox.Lang.String) as Toybox.Lang.Number or Null {
        return text.toNumber();
    }

    function numberOrZero(value as Toybox.Lang.Number or Null)
            as Toybox.Lang.Number {
        if (value == null) { return 0; }
        return value;
    }

    function textOrFallback(value as Toybox.Lang.String or Null)
            as Toybox.Lang.String {
        if (value == null) { return "fallback"; }
        return value;
    }
}
