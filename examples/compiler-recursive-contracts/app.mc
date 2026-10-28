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
typedef AppContract as interface {
    function ownValue() as Lang.Number;
    function again() as AppContract;
};
typedef Record as {:next as Record, :label as Lang.Number};
typedef WideRecord as {:next as WideRecord, :label as Lang.Number, :extra as Lang.Number};
module Checks {
    function makeRecord() as WideRecord { return {:label=>41, :extra=>7}; }
    function narrowRecord(value as WideRecord) as Record { return value; }
    function recordLabel(value as Record) as Lang.Number {
        var label = value[:label];
        if (label == null) { return -1; }
        return label;
    }
    function nextRecordLabel(value as Record) as Lang.Number {
        var next = value[:next];
        if (next == null) { return -1; }
        return Checks.recordLabel(next);
    }
    function eraseApp(app as Application.AppBase) as Application.AppBase { return app; }
    function actualAppValue(object as Lang.Object) as Lang.Number {
        if (object instanceof MinimalConsoleApp) { return object.ownValue(); }
        return -1;
    }
    function appValue(app as AppContract) as Lang.Number {
        return app.again().ownValue();
    }
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
    var appCount as Lang.Number;
    var savedApp as MinimalConsoleApp or Null;
    var savedInterface as AppContract or Null;
    function initialize() {
        AppBase.initialize();
        self.appCount = 17;
        self.savedApp = self;
        self.savedInterface = self;
    }
    function ownValue() as Lang.Number { return self.appCount; }
    function again() as AppContract { return self; }
    function asApp() as MinimalConsoleApp { return self; }
    function bumpApp() as Void { self.appCount += 1; }
    function forwardApp() as Lang.Number {
        var copy = self;
        return Checks.appValue(copy);
    }
    function savedValue() as Lang.Number {
        var saved = self.savedApp;
        if (saved == null) { return -1; }
        return saved.ownValue();
    }
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
        System.println(self.forwardApp());
        var app = self.asApp();
        app.bumpApp();
        System.println(Checks.appValue(app));
        var savedApplication = self.savedInterface;
        if (savedApplication == null) { System.println(-1); }
        else { System.println(savedApplication.ownValue()); }
        System.println(self.savedValue());
        System.println(Checks.actualAppValue(Checks.eraseApp(self)));
        System.println(Checks.actualAppValue(new Node()));
        var wide = Checks.makeRecord();
        var record = wide as Record;
        System.println(Checks.recordLabel(Checks.narrowRecord(wide)));
        var child = Checks.makeRecord();
        wide[:next] = child;
        System.println(Checks.nextRecordLabel(record));
        child[:label] = 42;
        System.println(Checks.nextRecordLabel(record));
        record[:label] = 43;
        System.println(Checks.recordLabel(wide));
        wide[:next] = wide;
        System.println(Checks.nextRecordLabel(record));
        System.exit();
    }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
