using Toybox.System;

function start() {
    System.println(0);
    System.println(127);
    System.println(128);
    System.println(255);
    System.println(256);
    System.println(32767);
    System.println(32768);
    System.println(65535);
    System.println(65536);
    System.println(8388607);
    System.println(8388608);
    System.println(2147483647);
    System.println(-1);
    System.println(-128);
    System.println(-129);
    System.println(-32768);
    System.println(-32769);
    System.println(-8388608);
    System.println(-8388609);
    System.println(-2147483648);
    System.println((-128).abs());
    System.println(Numbers.identity(-2147483648));
    System.println(Numbers.next(32767));
    System.println(1 - -2);
    var value = -129;
    value = 65536;
    System.println(value);
    System.println(2147483647 > 8388608);
    System.println(-2147483648 == Numbers.identity(-2147483648));
    System.exit();
}
