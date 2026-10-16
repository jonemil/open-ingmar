module Values {
    function boolean(present as Toybox.Lang.Boolean) as Toybox.Lang.Boolean or Null {
        return present ? false : null;
    }

    function booleanOrTrue(value as Toybox.Lang.Boolean or Null) as Toybox.Lang.Boolean {
        if (value == null) { return true; }
        return value;
    }

    function character(present as Toybox.Lang.Boolean) as Toybox.Lang.Char or Null {
        return present ? 'A' : null;
    }

    function charOrDefault(value as Toybox.Lang.Char or Null) as Toybox.Lang.Char {
        if (null == value) { return '?'; }
        return value;
    }

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
