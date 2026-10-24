using Toybox.Lang;
using Toybox.System;

typedef Pair as [Lang.Number,Lang.String];
typedef Reverse as Method() as Lang.Array<Lang.Object or Null>;
typedef Slice as Method(start as Lang.Number or Null,end as Lang.Number or Null) as Lang.Array<Lang.Object or Null>;
class Token {function number() as Lang.Number {return 41;}}
class Order {
    function compare(a as Lang.Object,b as Lang.Object) as Lang.Number {
        if(a instanceof Lang.Number && b instanceof Lang.Number){return a-b;}
        return 0;
    }
}
module Copies {
    function reversed(x as Pair) as [Lang.String,Lang.Number] {return x.reverse();}
    function first(x as Pair) as [Lang.Number] {return x.slice(0,1);}
    function last(x as Pair) as [Lang.String] {return x.slice(-1,null);}
    function all(x as Pair) as Pair {return x.slice(null,null);}
    function clipped(x as Pair) as Pair {return x.slice(-99,99);}
    function dynamic(x as Pair,i as Lang.Number) as Lang.Number {return x.slice(i,null).size();}
    function source(x as [Token,Lang.String]) as Lang.Number {return x.reverse()[1].number();}
    function nested(x as [Lang.Number,[Lang.String]]) as Lang.String {return x.slice(1,null)[0][0];}
    function sorted(x as [Lang.Number,Lang.Number,Lang.Number]) as [Lang.Number,Lang.Number,Lang.Number] {x.sort(null);return x;}
    function ordered(x as [Lang.Number,Lang.Number,Lang.Number]) as [Lang.Number,Lang.Number,Lang.Number] {x.sort(new Order());return x;}
    function reverseMethod(x as Pair) as Reverse {return x.method(:reverse);}
    function sliceMethod(x as Pair) as Slice {return x.method(:slice);}
    function weak(x as Pair) as Lang.Boolean {
        var handle=x.weak();
        var alive=handle.get()!=null;
        System.println(x.size());
        return alive;
    }
    function receiver() as Pair {System.println("receiver");return [1,"item"];}
    function begin() as Lang.Number {System.println("begin");return 1;}
    function end() as Lang.Number {System.println("end");return 2;}
}
class MinimalConsoleApp extends Toybox.Application.AppBase {
    function initialize(){AppBase.initialize();}
    function onStart(state){
        System.println("tuple-methods");
        var x=[3,"last"];
        var reversed=Copies.reversed(x);
        System.println(reversed[0]);
        reversed[1]=99;
        System.println(reversed[1]);
        System.println(x[0]);
        var first=Copies.first(x);
        first[0]=8;
        System.println(first[0]);
        System.println(x[0]);
        System.println(Copies.last(x)[0]);
        System.println(Copies.all(x).size());
        System.println(Copies.clipped(x).size());
        System.println(x.slice(2,0).size());
        System.println(Copies.dynamic(x,1));
        System.println(Copies.dynamic(x,0));
        System.println(Copies.source([new Token(),"source"]));
        System.println(Copies.nested([1,["nested"]]));
        System.println(x.indexOf("last"));
        System.println(x.indexOf(true));
        var order=Copies.sorted([3,1,2]);
        System.println(order[0]);
        System.println(order[2]);
        var custom=Copies.ordered([2,3,1]);
        System.println(custom[0]);
        System.println(custom[2]);
        var size=x.method(:size);
        System.println(size.invoke());
        var bound=Copies.reverseMethod(x);
        var item=bound.invoke()[0];
        if(item instanceof Lang.String){System.println(item);}
        var slice=Copies.sliceMethod(x);
        System.println(slice.invoke(0,1).size());
        System.println(Copies.weak(x));
        System.println(Copies.receiver().slice(Copies.begin(),Copies.end()).size());
    }
    function getInitialView(){return [new HelloView()];}
}
class HelloView extends Toybox.WatchUi.View {
    function initialize(){View.initialize();}
    function onUpdate(dc){System.exit();}
}
