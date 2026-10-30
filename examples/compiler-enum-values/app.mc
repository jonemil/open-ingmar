using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
enum { FLOAT=1.5f, LONG=4294967297l, DOUBLE=2.5d, TEXT="hello", FLAG=true,
       LETTER='A', SYMBOL=:tag, NOTHING=null, ARRAY=[1,2], DICT={:x=>3}, AFTER }
module M {
    enum { TEXT="module", AFTER }
    function value() as Lang.String { return TEXT; }
}
class Token {
    function initialize() {}
    function value() { return 23; }
}
class Box {
    enum { FLOAT=1.5f, LONG=4294967297l, DOUBLE=2.5d, TEXT="class",
           FLAG=true, LETTER='B', SYMBOL=:tag, NOTHING=null, ARRAY=[1,2],
           EMPTY=[], DICT={:x=>3}, TOKEN=new Token(), AFTER }
    function initialize() {}
    function token() as Token { return self.TOKEN; }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(FLOAT instanceof Lang.Float);
        System.println(LONG == 4294967297l);
        System.println(DOUBLE == 2.5d);
        System.println(TEXT.length());
        System.println(FLAG);
        System.println(LETTER == 'A');
        System.println(SYMBOL == :tag);
        System.println(NOTHING == null);
        System.println(ARRAY.size());
        System.println(DICT[:x] == 3);
        System.println(AFTER);
        System.println(M.value().length());
        System.println(M.AFTER);
        System.println(Box.FLOAT == 1.5f);
        System.println(Box.LONG == 4294967297l);
        System.println(Box.DOUBLE == 2.5d);
        System.println(Box.TEXT.length());
        System.println(Box.FLAG);
        System.println(Box.LETTER == 'B');
        System.println(Box.SYMBOL == :tag);
        System.println(Box.NOTHING == null);
        System.println(Box.ARRAY.size());
        System.println(Box.EMPTY == null);
        System.println(Box.DICT == null);
        System.println(Box.TOKEN == null);
        Box.ARRAY.add(3);
        var a = new Box();
        var saved = a.token();
        var b = new Box();
        System.println(a.ARRAY.size());
        System.println(b.ARRAY.size());
        System.println(Box.EMPTY.size());
        System.println(Box.DICT[:x] == 3);
        System.println(Box.TOKEN.value());
        System.println(saved.value());
        System.println(Box.AFTER);
        System.exit();
    }
}
