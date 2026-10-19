# Numeric alias arithmetic

2026-10-19. Question: can Numeric results, including ByteArray decoding, be
used directly in arithmetic without conversion or a concrete class guard?

**Publicly documented:** [Lang typedefs][lang] define Integer as Number/Long,
Decimal as Float/Double, and Numeric as all four types. The
[language reference][language] supplies scalar arithmetic semantics; typedefs
do not imply extra runtime wrapper objects.

**Experimentally observed:** SDK/compiler/simulator 9.2.0, Forerunner 265,
API target 5.2.0, minimum 3.1.0 accept
[the authored example](../../examples/compiler-numeric-unions/app.mc).
All 16 concrete argument pairings work through Numeric parameters. Sum values
and runtime classes follow the existing scalar promotion table: Number/Long
remain integral, Float with Number/Float remains Float, and Float with Long or
any Double operand produces Double. Subtraction, multiplication, division, and
comparison likewise execute through the Numeric signature. Integer supports
bitwise/remainder/shift operations; Decimal accepts floating arithmetic.
Nullable signatures, direct guards, compound writes, and updates compile and
run. Negating each of the four classes through one Numeric function preserves
its result width. UINT32 decoding followed by `+ 1` yields Long `4294967296`.

The generated application and SDK application match all
[96 expected console lines](../../examples/compiler-numeric-unions/expected.txt).
Both PRGs support structured CODE/DATA export and reconstruction; generated
source, saved IR, and assembly paths produce the same unsigned application.
The compiler's [current alias contracts](../specs/compiler-subset.md#numeric-values)
remain bounded; this specimen does not establish arbitrary union storage or
runtime error behavior.

For SDK reproduction, reuse the minimal-console manifest/resources and point
`base.sourcePath` to `../../examples/compiler-numeric-unions` from
`scratch/compiler-numeric-unions/monkey.jungle`. The example contains its own
AppBase class. Build with `scripts/build-example.sh`, then use
`scripts/check-example.py prepare compiler-numeric-unions --sdk-prg <SDK PRG>
--output <fresh directory>` and `simulate <prepared directory>` in the Rust and
SDK environments. Current captures are in `scratch/check-numeric-unions-v2/`.

[lang]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang.html
[language]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/
