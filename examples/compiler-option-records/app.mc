using Toybox.Application;
using Toybox.Lang;
using Toybox.System;

typedef Conversion as {:offset as Lang.Number, :endianness as Lang.Endian};
typedef Named as {:name as Lang.String, :count as Lang.Number};
class Holder {
    var options as Conversion;
    function initialize() { self.options = Options.make(); }
    function read() as Lang.Number {
        var offset = self.options[:offset];
        if (offset != null) { return offset; }
        return -1;
    }
}
module Options {
    function make() as Conversion { return {:offset=>0, :endianness=>Lang.ENDIAN_BIG}; }
    function empty() as Conversion { return {}; }
    function named() as Named { return {:name=>"record", :count=>1, :extra=>true}; }
    function nested() as {:inner as Conversion} { return {:inner=>{:offset=>2}}; }
    function choose(flag as Lang.Boolean) as Conversion { return flag ? {:offset=>1} : {}; }
    function update(options as Conversion, offset as Lang.Number) as Conversion {
        options[:offset] = offset;
        return options;
    }
    function run() as Void {
        var bytes = [1, 2, 3, 4]b;
        var options = make();
        update(options, 1);
        System.println(bytes.decodeNumber(Lang.NUMBER_FORMAT_UINT8, options));
        options.put(:offset, 0);
        bytes.encodeNumber(4660, Lang.NUMBER_FORMAT_UINT16, options);
        System.println(bytes[0]);
        System.println(bytes[1]);
        var offset = options.get(:offset);
        if (offset != null) { System.println(offset); }
        options.remove(:offset);
        System.println(options[:offset] == null);
        System.println(empty()[:offset] == null);
        var named = named();
        named[:count] = 3;
        var name = named[:name];
        if (name != null) { System.println(name); }
        var count = named[:count];
        if (count != null) { System.println(count); }
        var child = nested()[:inner];
        if (child != null) {
            var value = child[:offset];
            if (value != null) { System.println(value); }
        }
        var chosen = choose(true)[:offset];
        if (chosen != null) { System.println(chosen); }
        System.println(new Holder().read());
        var local = {:offset=>2, :endianness=>Lang.ENDIAN_BIG};
        System.println(bytes.decodeNumber(Lang.NUMBER_FORMAT_UINT8, local));
    }
}
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) { Options.run(); System.exit(); }
}
