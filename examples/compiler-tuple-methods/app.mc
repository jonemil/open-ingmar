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
typedef RemovalRecord as interface { var pair as Pair; };
class RemovalOwner {
    var pair as Pair;
    var optional as Pair or Null;
    function initialize(){self.pair=[1,"field"];}
    function removed() as Lang.Number {self.pair.remove(1);return self.pair.size();}
    function resetNeedle() as Lang.Number {self.pair=[7,"new"];return 1;}
    function captured() as Lang.Number {self.pair.remove(self.resetNeedle());return self.pair.size();}
    function nullable() as Lang.Number {
        if(self.optional!=null){self.optional.removeAll(1);return self.optional.size();}
        return 0;
    }
}
module Removals {
    function one(x as Pair) as Lang.Number {System.println(x.remove(1));return x.size();}
    function all(x as [Lang.Number,Lang.String,Lang.Number]) as Lang.Number {
        System.println(x.removeAll(1));
        System.println(x.removeAll(99));
        return x.size();
    }
    function miss(x as Pair) as Lang.Number {System.println(x.remove(false));return x.size();}
    function aliased(x as Pair) as Pair {var old=x;x.remove(1);return old;}
    function rebound(x as Pair) as Pair {x.remove(1);x=[5,"new"];return x;}
    function callee(x as Pair) as Lang.Boolean {return x.remove(1);}
    function caller(x as Pair) as Pair {callee(x);return x;}
    function bound(x as Pair) as Pair {var remove=x.method(:remove);System.println(remove.invoke(1));return x;}
    function branch(x as Pair,b as Lang.Boolean) as Lang.Number {if(b){x.remove(1);}return x.size();}
    function repeated(x as [Lang.Number,Lang.String,Lang.Number],n as Lang.Number) as Lang.Number {
        for(var i=0;i<n;i++){x.remove(1);}
        return x.size();
    }
    function guarded(x as Pair or Null) as Lang.Number {if(x!=null){x.removeAll(1);return x.size();}return 0;}
    function staged(x as Pair,b as Lang.Boolean) as Lang.Number {System.println(x.remove(b?1:2));return x.size();}
    function erasedArgument(x as Pair) as [Lang.Number,Lang.String,Lang.Boolean] {x.add(x.remove(1));return x;}
    function indexedArgument(x as Pair) as [Lang.Boolean,Lang.String] {x[0]=x.remove(1);return x;}
    function appended(x as Pair) as Lang.Number {System.println(x.remove(x.add(2).size()));return x.size();}
    function other(x as RemovalOwner) as Pair {x.pair.remove(1);return x.pair;}
    function structural(x as RemovalRecord) as Lang.Number {x.pair.removeAll(1);return x.pair.size();}
    function grouped(x as RemovalRecord,y as RemovalRecord) as Lang.Number {x.pair.remove(1);return y.pair.size();}
    function nested(x as [Pair,Lang.Boolean]) as Lang.Number {x[0].remove(1);return x[0].size();}
    function source(x as [Token,Lang.String],token as Token) as Lang.Number {System.println(x.remove(token));return x.size();}
    function receiver(x as Pair) as Pair {System.println("remove-receiver");return x;}
    function needle() as Lang.Number {System.println("remove-needle");return 1;}
    function run() as Void {
        System.println("removals");
        System.println(one([1,"one"]));
        System.println(all([1,"all",1]));
        System.println(miss([1,"miss"]));
        System.println(aliased([1,"alias"]).size());
        System.println(rebound([1,"old"])[0]);
        System.println(caller([1,"callee"]).size());
        System.println(bound([1,"bound"]).size());
        System.println(branch([1,"branch"],true));
        System.println(branch([1,"branch"],false));
        System.println(repeated([1,"loop",1],0));
        System.println(repeated([1,"loop",1],2));
        System.println(guarded(null));
        System.println(guarded([1,"nullable"]));
        System.println(staged([1,"staged"],true));
        System.println(appended([1,"append"]));
        System.println(erasedArgument([1,"capture"]).size());
        System.println(indexedArgument([1,"indexed"]).size());
        System.println(new RemovalOwner().removed());
        System.println(new RemovalOwner().captured());
        System.println(other(new RemovalOwner()).size());
        System.println(structural(new RemovalOwner()));
        System.println(grouped(new RemovalOwner(),new RemovalOwner()));
        System.println(nested([[1,"nested"],true]));
        var token=new Token();
        System.println(source([token,"source"],token));
        var owner=new RemovalOwner();
        System.println(owner.nullable());
        owner.optional=[1,"optional"];
        System.println(owner.nullable());
        System.println(receiver([1,"effects"]).remove(needle()));
    }
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
        Removals.run();
    }
    function getInitialView(){return [new HelloView()];}
}
class HelloView extends Toybox.WatchUi.View {
    function initialize(){View.initialize();}
    function onUpdate(dc){System.exit();}
}
