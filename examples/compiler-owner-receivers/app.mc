using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
var VALUE as Lang.Number=3;
const LITERAL=23;
function literal() as Lang.Number{return self.LITERAL+me.LITERAL;}
function globalPath() as Lang.Number{return me.Outer.Inner.Child.value();}
function root() as Lang.Number {return self.VALUE+me.VALUE;}
function globalCall() as Lang.Number {return 5;}
module Outer {
 var VALUE as Lang.Number=7;
 function parent() as Lang.Number{return 11;}
 module Inner {
  var VALUE as Lang.Number=13;
  function read() as Lang.Number{return self.VALUE+me.VALUE;}
  function inherited() as Lang.Number{return self.parent()+me.parent()+self.globalCall()+me.globalCall();}
  function write(){self.VALUE+=1;me.VALUE+=2;}
  function child() as Lang.Number{return self.Child.value()+me.Child.value();}
  module Child{function value() as Lang.Number{return 17;}}
 }
}
class Token {
 var value as Lang.Number;
 function initialize(n as Lang.Number){me.value=n;}
 function get() as Lang.Number{return me.value+self.value;}
 function identity() as Token{return me;}
 function globalMember() as Lang.Number{return me.VALUE+self.globalCall();}
 function bind() as Lang.Method {return me.method(:get);}
}
module Trace {
 function mark(n as Lang.Number) as Lang.Number {System.println(n);return n;}
}
module Cold {
 var MARK as Lang.Number=Trace.mark(101);
 module Target {
  var MARK as Lang.Number=Trace.mark(102);
  module Deep {
   var MARK as Lang.Number=Trace.mark(103);
   function value(n as Lang.Number) as Lang.Number{return n;}
   function touch() as Void {System.println(106);}
  }
 }
 module Peer {
  var MARK as Lang.Number=Trace.mark(104);
  function run() as Lang.Number {return me.Target.Deep.value(Trace.mark(105));}
  function touch() {self.Target.Deep.touch();}
 }
}
class MinimalConsoleApp extends Application.AppBase {
 function initialize(){AppBase.initialize();}
 function onStart(state){
  System.println(root());
  System.println(literal());
  System.println(Outer.Inner.read());
  System.println(Outer.Inner.inherited());
  Outer.Inner.write();
  System.println(Outer.Inner.read());
  System.println(Outer.Inner.child());
  System.println(globalPath());
  var token=new Token(19);
  System.println(token.get());
  System.println(token.identity().get());
  System.println(token.globalMember());
  var callback=token.bind();
  System.println(callback.invoke() as Lang.Number);
  System.println(Cold.Peer.run());
  Cold.Peer.touch();
  System.exit();
 }
}
