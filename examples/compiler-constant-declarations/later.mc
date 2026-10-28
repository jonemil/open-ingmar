using Toybox.Lang;
typedef Count as Lang.Number;
typedef MaybeToken as Token or Null;
class Token {
    const VALUE as Lang.String = "token";
    function text() as Lang.String { return VALUE; }
}
