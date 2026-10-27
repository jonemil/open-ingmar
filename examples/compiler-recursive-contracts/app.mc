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
    function next() as Recursive { return new Node(); }
    function value() as Lang.Number { return 41; }
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
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
