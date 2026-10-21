// Repeated reads distinguish local storage from one-use expression values.
function probe() as Toybox.Lang.Number {
    var first = 40 + 2;
    var second = first - 5;
    return first + second - first;
}
