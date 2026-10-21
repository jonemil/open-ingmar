// The scaffold supplies Numbers.left/right and the difference method metadata.
function probe() as Toybox.Lang.Number {
    var result = Numbers.difference(Numbers.left() + 1, Numbers.right());
    return result + 1;
}
