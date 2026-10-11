module Slices {
    function orFallback(text as Toybox.Lang.String,
                        start as Toybox.Lang.Number,
                        end as Toybox.Lang.Number) as Toybox.Lang.String {
        var piece = text.substring(start, end);
        if (piece == null) { return "invalid"; }
        return piece;
    }
}
