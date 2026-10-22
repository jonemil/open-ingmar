using Toybox.Application;
using Toybox.System;
import Toybox.Lang;

class Token {
    function initialize() {}
    function answer() as Number { return 7; }
}

class AliasBox {
    typedef Item as Shared;
    var value as Item or Null;
    function store(item as Item) as Void { self.value = item; }
    function read() as Item or Null { return self.value; }
}

module AliasUse {
    typedef Scalar as Types.Element;
    function identity(value as Shared) as Shared { return value; }
    function token(value as OptionalToken) as OptionalToken { return value; }
    function number(value) as Scalar { return value as Scalar; }
    function run() as Void {
        System.println("typedefs");
        System.println(AliasUse.identity(41));
        System.println(AliasUse.identity("text"));
        var array = Types.array([2, 3]);
        System.println(array[0] + array[1]);
        var dictionary = Types.dictionary({"answer" => 42});
        var answer = dictionary["answer"];
        if (answer != null) { System.println(answer); }
        var token = AliasUse.token(new Token());
        if (token != null) { System.println(token.answer()); }
        System.println(AliasUse.number(9));
        var box = new AliasBox();
        System.println(box.read() == null);
        box.store("field");
        var saved = box.read();
        if (saved != null) { System.println(saved); }
    }
}

class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { AliasUse.run(); System.exit(); }
    function getInitialView() { return [new Toybox.WatchUi.View()]; }
}
