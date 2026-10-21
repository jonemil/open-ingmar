// Positional parameters occupy slots before this initialized local.
function difference(first as Toybox.Lang.Number, second as Toybox.Lang.Number) as Toybox.Lang.Number {
    var adjusted = first + 3;
    return adjusted - second;
}
