using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
module Trace {
 var LOG as Lang.Number = 0;
 function mark(n as Lang.Number) as Lang.Number { LOG = LOG * 10 + n; return n; }
}
class Counter {
 static var value as Lang.Number = Trace.mark(2);
 static var literal as Lang.Number = 7;
 static const LABEL = "static";
 static var absent as Lang.Number or Null;
 static function add(n as Lang.Number) as Lang.Number { value += n; return value; }
 static function owner() { return self; }
 var own as Lang.Number = Trace.mark(3);
 function initialize() { System.println(own); }
 function bump() as Lang.Number { return add(1); }
}
class MinimalConsoleApp extends Application.AppBase {
 function initialize(){ AppBase.initialize(); }
 function onStart(state as Lang.Dictionary or Null) as Void {
  System.println(Trace.LOG);
  // A class reference does not activate the separate static storage.
  var klass = Counter;
  System.println(Trace.LOG);
  System.println(klass == Counter);
  System.println(Counter.literal);
  System.println(Counter.absent == null);
  System.println(Counter.LABEL);
  System.println(Trace.LOG);
  System.println(Counter.value);
  System.println(Counter.add(4));
  Counter.value = 10;
  var c = new Counter();
  System.println(c.bump());
  var other = new Counter();
  System.println(Counter.value);
  System.println(Counter.owner() == Counter);
  // Static self is callable storage, distinct from the class-definition value.
  var m = new Lang.Method(Counter.owner(),:add);
  System.println(m.invoke(2));
  System.println(Counter.owner() == Counter);
  System.println(Trace.LOG);
  var instanceMethod = c.method(:add);
  System.println(instanceMethod.invoke(1));
  System.exit();
 }
}
