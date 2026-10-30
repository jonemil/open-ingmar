using Toybox.Application;
using Toybox.Lang;
using Toybox.System;
enum Floats { F=1.5f }
enum Texts { TEXT="hello", COPY=TEXT }
enum Mixed { WORD="mixed", FRACTION=2.5f }
enum WithNumber { STR="text", REAL=1.5f, IMPLICIT }
enum Nulls { EMPTY=null }
enum Arrays { ITEMS=[3,4] }
enum Dictionaries { TABLE={:x=>3} }
enum Flags { YES=true }
class Token {
    function initialize() {}
    function value() as Lang.Number { return 23; }
}
enum Objects { ITEM=new Token() }
module M {
    enum Kind { TEXT="module" }
    function value() as Kind { return TEXT; }
}
class C {
    enum Kind { TEXT="class" }
    function initialize() {}
    function value() as Kind { return self.TEXT; }
}
function floatValue(value as FloatChoice) as Lang.Float { return value; }
function textLength(value as Texts) { return value.length(); }
function mixedLength(value as Mixed) { return value.length(); }
function mixedFloat(value as Mixed) as Lang.Float { return value; }
function number(value as WithNumber) as Lang.Number { return value; }
function echo(value as Texts) as Texts { return value; }
function makeTexts() as Lang.Array<Texts> { return [TEXT]; }
function arraySize(value as Arrays) { return value.size(); }
function objectValue(value as Objects) as Lang.Number { return value.value(); }
function nullValue(value as Nulls) as Nulls { return value; }
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(floatValue(F));
        System.println(F + 1);
        System.println(-F);
        System.println(textLength(TEXT));
        System.println(echo(COPY).length());
        System.println(mixedLength(WORD));
        System.println(mixedFloat(FRACTION));
        System.println(number(IMPLICIT));
        System.println(nullValue(EMPTY) == null);
        System.println(arraySize(ITEMS));
        System.println(M.value().length());
        System.println(C.TEXT.length());
        var instance = new C();
        System.println(instance.value().length());
        var values = makeTexts();
        values.add(COPY);
        System.println(values[1].length());
        var table = {TEXT => COPY} as Lang.Dictionary<Texts, Texts>;
        var found = table[TEXT];
        if (found != null) { System.println(found.length()); }
        ITEMS[0]=9;
        System.println(ITEMS[0]);
        TABLE[:x]=9;
        System.println(TABLE[:x] == 9);
        if (YES) { System.println(true); } else { System.println(false); }
        System.println(objectValue(ITEM));
        System.exit();
    }
}
