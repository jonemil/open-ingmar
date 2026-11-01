using Toybox.Lang;
using Toybox.System;
module Library {
    module Models {
        typedef Amount as Lang.Number;
        enum Mode { READY=4 }
        var COUNT as Amount=0;
        var DATA as Lang.Array<Amount> = [1,2];
        class Token {
            enum Kind { ON=3 }
            var value as Amount;
            function initialize(n as Amount) { value=n; }
            function get() as Amount { return value; }
        }
        function make(n as Amount) as Token { return new Token(n); }
        function bump() as Amount { COUNT+=1; return COUNT; }
        module Nested {
            function answer() as Lang.Number { return 23; }
        }
    }
}
module Consumer {
    function extra() as Exact { return new Library.Models.Token(8); }
}
