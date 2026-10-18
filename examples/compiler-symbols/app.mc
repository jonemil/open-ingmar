using Toybox.Application;
using Toybox.System;
module Symbols {
    function echo(s as Toybox.Lang.Symbol) as Toybox.Lang.Symbol { return s; }
    function run() as Void {
        SymbolFieldChecks.run();
        SymbolContainers.run();
        NullableSymbols.run();
        var a = :alpha;
        var b = :alpha;
        var c = :beta;
        System.println(a == b); System.println(a != c);
        System.println(a.equals(b)); System.println(a.equals(c));
        System.println(a.equals(null)); System.println(a.equals("alpha")); System.println(a.equals(a.toNumber()));
        System.println(a.hashCode() == b.hashCode());
        System.println(a.toNumber() == b.toNumber());
        System.println(a.toNumber() != c.toNumber());
        System.println(a.toString());
        System.println(Symbols.echo(a).equals(a));
        System.println(a.weak().equals(a));
        System.println(:size.toString());
    }
}
module NullableSymbols {
    function value(present as Toybox.Lang.Boolean) as Toybox.Lang.Symbol or Null {return present ? :alpha : null;}
    function reverse(present as Toybox.Lang.Boolean) as Toybox.Lang.Symbol or Null {return present ? null : :beta;}
    function forward(value as Toybox.Lang.Symbol or Null) as Toybox.Lang.Symbol or Null {return value;}
    function name(value as Toybox.Lang.Symbol or Null) as Toybox.Lang.String {
        if (null == value) {return "missing";}
        return value.toString();
    }
    function selector(present as Toybox.Lang.Boolean) as Toybox.Lang.Symbol or Null {return present ? :toString : null;}
    function run() as Void {
        System.println(NullableSymbols.value(false) == null);
        System.println(null != NullableSymbols.value(true));
        System.println(NullableSymbols.reverse(true) == null);
        System.println(NullableSymbols.name(NullableSymbols.forward(:alpha)));
        System.println(NullableSymbols.name(NullableSymbols.forward(null)));
        var value = NullableSymbols.value(true);
        if (value != null) {
            System.println(value.equals(:alpha));
            System.println(value.toNumber() == :alpha.toNumber());
            System.println(value.hashCode() == :alpha.hashCode());
            System.println(value.weak().equals(:alpha));
            System.println(value == :alpha);
        }
        var snapshot = value;
        value = null;
        System.println(NullableSymbols.name(value));
        System.println(NullableSymbols.name(snapshot));
        var visits = 0;
        while (snapshot != null) {
            System.println(snapshot.equals(:alpha));
            snapshot = null;
            visits = visits + 1;
        }
        System.println(visits);
        var selector = NullableSymbols.selector(true);
        if (selector != null) {System.println(1 has selector);}
        System.println(:alpha.equals(NullableSymbols.value(false)));
        System.println(:alpha.equals(NullableSymbols.value(true)));
        var selected = true ? NullableSymbols.reverse(false) : :alpha;
        System.println(NullableSymbols.name(selected));
    }
}
class MinimalConsoleApp extends Application.AppBase {
    var startupSymbol as Toybox.Lang.Symbol = :appMetadataOnly;
    function initialize() { AppBase.initialize(); }
    function onStart(state) {System.println(self.startupSymbol.toString()); Symbols.run(); System.exit();}
}

module SymbolContainers {
    function array() as Toybox.Lang.Array<Toybox.Lang.Symbol> {return [:alpha,:beta,:alpha,:size];}
    function empty() as Toybox.Lang.Array<Toybox.Lang.Symbol> {return [];}
    function optional() as Toybox.Lang.Array<Toybox.Lang.Symbol or Null> {return [null,:alpha];}
    function nulls() as Toybox.Lang.Array<Toybox.Lang.Symbol or Null> {return [null];}
    function dictionary() as Toybox.Lang.Dictionary<Toybox.Lang.Symbol,Toybox.Lang.Symbol or Null> {return {:alpha=>:beta,:beta=>null,:size=>:alpha};}
    function labels() as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.Symbol> {return {"a"=>:alpha};}
    function counts() as Toybox.Lang.Dictionary<Toybox.Lang.Symbol,Toybox.Lang.Number> {return {:alpha=>1,:beta=>2};}
    function run() as Void {
        var a=SymbolContainers.array();
        System.println(a[0].equals(:alpha));
        System.println(a[1].equals(:beta));
        System.println(a[2].equals(:alpha));
        System.println(a[3].equals(:size));
        var shared=a;
        var text=a.toString();
        a[1]=:size;
        a.add(:beta);
        a.addAll([:alpha]);
        System.println(shared.size());
        System.println(text!=a.toString());
        System.println(a.indexOf(:beta));
        System.println(a.remove(:alpha));
        System.println(a.removeAll(:alpha));
        System.println(shared.size());
        var reversed=a.reverse();
        System.println(reversed.slice(0,2)[0].equals(:beta));
        System.println(a.indexOf(:alpha));
        var empty=SymbolContainers.empty();
        empty.add(:alpha);
        System.println(empty[0].equals(:alpha));
        var optional=SymbolContainers.optional();
        System.println(optional[0]==null);
        optional[0]=:beta;
        optional.add(null);
        var snapshot=optional[1];
        optional[1]=null;
        if(snapshot!=null){System.println(snapshot.equals(:alpha));}
        System.println(optional.indexOf(null));
        System.println(optional.removeAll(null));
        System.println(optional.size());
        System.println(SymbolContainers.nulls()[0]==null);
        var d=SymbolContainers.dictionary();
        var value=d[:alpha];
        if(value!=null){System.println(value.toString());}
        System.println(d[:beta]==null);
        System.println(d.hasKey(:size));
        System.println(d[:missing]==null);
        d[:alpha]=:size;
        d.put(:beta,:alpha);
        var replaced=d.get(:alpha);
        if(replaced!=null){System.println(replaced.equals(:size));}
        var keys=d.keys();
        var values=d.values();
        d.remove(:size);
        System.println(keys.size());
        System.println(values.size());
        System.println(d.size());
        System.println(!d.hasKey(:size));
        var labels=SymbolContainers.labels();
        labels.put("b",:beta);
        System.println(labels.values().indexOf(:alpha)>=0);
        var counts=SymbolContainers.counts();
        var count=counts[:beta];
        if(count!=null){System.println(count);}
    }
}

class SymbolStore {
    var required as Toybox.Lang.Symbol;
    var maybe as Toybox.Lang.Symbol or Null;
    var explicitNull as Toybox.Lang.Symbol or Null = null;
    var preset as Toybox.Lang.Symbol = :metadataOnly;
    var optionalPreset as Toybox.Lang.Symbol or Null = :optionalMetadata;
    var selector = :size;
    function initialize(value as Toybox.Lang.Symbol) {self.required=value;}
    function read() as Toybox.Lang.Symbol {return self.required;}
    function replace(value as Toybox.Lang.Symbol) as Void {self.required=value;}
    function select(value as Toybox.Lang.Symbol or Null) as Void {self.maybe=value;}
    function isEmpty() as Toybox.Lang.Boolean {return self.maybe==null;}
    function nullDefault() as Toybox.Lang.Boolean {return self.explicitNull==null;}
    function presetName() as Toybox.Lang.String {return self.preset.toString();}
    function optionalName() as Toybox.Lang.String {
        var value=self.optionalPreset;
        if(value!=null){return value.toString();}
        return "missing";
    }
    function take() as Toybox.Lang.Symbol or Null {
        var value=self.maybe;
        self.maybe=null;
        return value;
    }
    function hasSize(value as Toybox.Lang.Array<Toybox.Lang.Number>) as Toybox.Lang.Boolean {return value has self.selector;}
}
module SymbolFieldChecks {
    function run() as Void {
        var first=new SymbolStore(:alpha);
        var second=new SymbolStore(:beta);
        System.println(first.read().equals(:alpha));
        System.println(first.isEmpty());
        System.println(first.nullDefault());
        System.println(first.presetName());
        System.println(first.optionalName());
        System.println(first.hasSize([1]));
        first.replace(:size);
        System.println(first.read().equals(:size));
        System.println(second.read().equals(:beta));
        first.select(:alpha);
        System.println(!first.isEmpty());
        var saved=first.take();
        System.println(first.isEmpty());
        if(saved!=null){System.println(saved.equals(:alpha));}
        System.println(first.take()==null);
    }
}
