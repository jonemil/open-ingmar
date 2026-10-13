using Toybox.System;
using Toybox.WatchUi;
module Views {
    function make() as Toybox.WatchUi.View {
        System.println("make");
        return new WatchUi.View();
    }
    function pass(view as Toybox.WatchUi.View) as Toybox.WatchUi.View {
        return view;
    }
    function empty() as Toybox.Lang.Array<Toybox.WatchUi.View> {
        return [];
    }
    function count(views as Toybox.Lang.Array<Toybox.WatchUi.View>) {
        return views.size();
    }
}
function start() {
    var view = Views.make();
    System.println("created");
    var views = [view, new WatchUi.View()];
    System.println(views.size());
    var shared = views;
    views.add(Views.pass(views[0]));
    System.println(shared.size());
    System.println(views.indexOf(view));
    System.println(views.remove(view));
    System.println(views.removeAll(view));
    System.println(views.size());
    views.addAll([new WatchUi.View(), new WatchUi.View()]);
    System.println(views.size());
    System.println(views.reverse().slice(0, 1).size());
    System.println(Views.count([Views.pass(views[0])]));
    views = [];
    System.println(views.size());
    System.println(shared.size());
    views.addAll([]);
    System.println(Views.empty().size());
    System.exit();
}
