using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
module Trace {function mark(n as Lang.Number) as Lang.Number{System.println(n);return n;}}
module A {
 var MARK as Lang.Number=Trace.mark(10);
 module B {
  var MARK as Lang.Number=Trace.mark(20);
  class Token {
   enum Kind { ON=3 }
   var value as Lang.Number=1;
   function initialize(n as Lang.Number){value=n;}
   function get() as Lang.Number{return value;}
   function owner() as Lang.Number{return MARK;}
  }
  function make() as Token{return new Token(42);}
 }
}
module Outer {
 var MARK as Lang.Number=Trace.mark(30);
 class Token {}
 module Inner {
  var MARK as Lang.Number=Trace.mark(40);
  module Deep {
   var MARK as Lang.Number=Trace.mark(50);
   function make() as Token { return new Token(); }
  }
 }
}
module Tests {
 var MARK as Lang.Number=Trace.mark(60);
 class Token {}
 module Inner {
  var MARK as Lang.Number=Trace.mark(70);
  module Deep {
   var MARK as Lang.Number=Trace.mark(80);
   function test(value as Lang.Object) as Lang.Boolean { return value instanceof Token; }
  }
 }
}
module Apps {
var MARK as Lang.Number=Trace.mark(90);
class MinimalConsoleApp extends Application.AppBase {
 function initialize(){AppBase.initialize();}
 function onStart(state){
  System.println("begin");
  var t=new A.B.Token(41);
  System.println(t.get());
  System.println(t.owner());
  System.println(t instanceof A.B.Token);
  System.println(A.B.make().get());
  System.println(A.B.forward(t).get());
  var other=new Other.Token();
  System.println(other.get());
  System.println(other instanceof A.B.Token);
  var copy=t as A.B.Token;
  System.println(copy.get());
  System.println(A.B.Token.ON);
  System.println(A.B.flag());
  System.println(Outer.Inner.Deep.make() instanceof Outer.Token);
  System.println(Tests.Inner.Deep.test(t));
  System.exit();
 }
}
}
