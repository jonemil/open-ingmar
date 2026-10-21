// The surrounding scaffold supplies zero-argument Numbers.left/right functions.
function probe() as Toybox.Lang.Number {
    var value = Numbers.left();
    return value + Numbers.right() - 39;
}
