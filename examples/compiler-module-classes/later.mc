using Toybox.Lang;
module A { module B {
 function flag() as Token.Kind { return Token.ON; }
 function forward(value as Token) as Token { return value; }
}}
module Other {
 class Token {
  function get() as Lang.Number { return 9; }
 }
}
