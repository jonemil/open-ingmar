using Toybox.Application;
using Toybox.Lang;
using Toybox.System;

class Plain {
    public var VALUE as Lang.Number = 40;
}
class Vault {
    private var VALUE as Lang.Number = 7;
    protected var PROTECTED as Lang.Number = 8;
    hidden var HIDDEN as Lang.Number = 9;
    public var OPEN as Lang.Number = 10;
    private static var STATIC as Lang.Number = 11;
    private const K = 4;
    hidden enum { E = 6 }
    private function secret() as Lang.Number { return VALUE; }
    protected function guarded() as Lang.Number { return PROTECTED; }
    hidden function hiddenCall() as Lang.Number { return HIDDEN; }
    private static function bump(n as Lang.Number) as Lang.Number {
        STATIC += n;
        return STATIC;
    }
    public function read() as Lang.Number {
        VALUE += 1;
        self.VALUE += 1;
        return secret() + guarded() + hiddenCall();
    }
    public function mixed() as Lang.Number {
        var other = new Plain();
        return other.VALUE + VALUE + K + E;
    }
    public function visible() as Lang.Number { return PROTECTED; }
    public function callback() as Lang.Number {
        var bound = method(:visible);
        return bound.invoke();
    }
    public static function readStatic() as Lang.Number {
        return Vault.bump(true ? 1 : 2);
    }
}
class Factory {
    private var value as Lang.Number;
    private function initialize(n as Lang.Number) { value = n; }
    public static function create(n as Lang.Number) as Factory { return new Factory(n); }
    public function get() as Lang.Number { return value; }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state as Lang.Dictionary or Null) as Void {
        var v = new Vault();
        System.println(v.OPEN);
        System.println(v.read());
        System.println(Vault.readStatic());
        System.println(v.mixed());
        System.println(v.callback());
        System.println(v has :VALUE);
        System.println(v has :secret);
        System.println(v has :OPEN);
        var factory = Factory.create(17);
        System.println(factory.get());
        System.exit();
    }
}
