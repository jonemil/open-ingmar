using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
typedef Tree as Lang.Array<Tree>;
typedef MaybeTree as Lang.Array<MaybeTree> or Null;
typedef Map as Lang.Dictionary<Lang.String,Map>;
typedef Recursive as interface {
    function next() as Recursive;
    function value() as Lang.Number;
};
module Checks {
    function emptyMap() as Map { return {}; }
    function branchMap(child as Map) as Map { return {"child"=>child}; }
    function mapSize(map as Map) as Lang.Number {
        var child = map["child"];
        if (child == null) { return -1; }
        return child.size();
    }
    function nullableSize(tree as MaybeTree) as Lang.Number {
        if (tree == null) { return -1; }
        return tree.size();
    }
    function child(tree as Tree) as Tree { return tree[0]; }
    function size(tree as Tree) as Lang.Number { return tree[0].size(); }
    function leaf() as Tree { return []; }
    function branch(tree as Tree) as Tree { return [tree]; }
    function chain(node as Recursive) as Lang.Number { return node.next().value(); }
}
class Node {
    var count as Lang.Number;
    var cached as Recursive or Null;
    function initialize() { self.count = 41; }
    function next() as Recursive { return self; }
    function value() as Lang.Number { return self.count; }
    function bump() as Void { self.count += 1; }
    function asNode() as Node { return self; }
    function forward() as Lang.Number { return Checks.chain(self); }
    function remember() as Recursive {
        self.cached = self;
        var copy = self;
        return copy;
    }
    function rememberedValue() as Lang.Number {
        var saved = self.cached;
        if (saved == null) { return -1; }
        return saved.value();
    }
}
class Holder {
    var current as Recursive;
    function initialize(node as Recursive) { self.current = node; }
    function follow() as Lang.Number { return self.current.next().value(); }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        var leaf = Checks.leaf();
        var branch = Checks.branch(leaf);
        System.println(Checks.size(branch));
        System.println(Checks.child(branch).size());
        System.println(Checks.chain(new Node()));
        System.println(new Holder(new Node()).follow());
        System.println(Checks.mapSize(Checks.branchMap(Checks.emptyMap())));
        System.println(Checks.mapSize(Checks.emptyMap()));
        System.println(Checks.nullableSize(null));
        System.println(Checks.nullableSize([]));
        var node = new Node();
        System.println(node.forward());
        var saved = node.remember();
        System.println(saved.value());
        node.asNode().bump();
        System.println(saved.next().value());
        System.println(node.rememberedValue());
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
