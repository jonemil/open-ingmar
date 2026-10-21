function probe() as Toybox.Lang.Number {
    var result = Numbers.difference(Numbers.left() + 1, Numbers.right());
    System.println(result);
    return result + 1;
}
