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
[expected console lines](../../examples/compiler-numeric-unions/expected.txt).
Both PRGs support structured CODE/DATA export and reconstruction; generated
source, saved IR, and assembly paths produce the same unsigned application.
The compiler's [current alias contracts](../specs/frontend/README.md#numbers-and-numeric-aliases)
retain the scalar promotion rules; this does not establish runtime error behavior.

For SDK reproduction, reuse the minimal-console manifest/resources and point
`base.sourcePath` to `../../examples/compiler-numeric-unions` from
`scratch/compiler-numeric-unions/monkey.jungle`. The example contains its own
AppBase class. Build with `scripts/build-example.sh`, then use
`scripts/check-example.py prepare compiler-numeric-unions --sdk-prg <SDK PRG>
--output <fresh directory>` and `simulate <prepared directory>` in the Rust and
SDK environments. Current captures are in `scratch/check-numeric-unions-v2/`.

## Native union signatures and narrowing

The same authored example now includes `NativeUnions`. SDK/compiler/simulator
9.2.0 / fr265 accept Number-or-String signatures, common `toString`, arithmetic
with union results, both branches of `instanceof`, Null-first three-way unions,
standalone Null signatures, nullable Array references, and loop-exit narrowing.
Char equality with Null evaluates false. These observations concern our authored
source, not the SDK/compiler implementation.

SDK and generated apps match all 114 expected console lines. Source, saved IR,
and saved assembly produce the same unsigned generated PRG. Both SDK and
generated PRGs pass structured CODE/DATA export and byte-preserving unsigned
reconstruction; no required raw fallback occurs. Current captures are in
`scratch/unions/check/`; reproduce with the specimen checker commands above.
This specimen does not establish exact source-class unions or union fields.

## Field storage

SDK/compiler/simulator 9.2.0 / fr265 also accept
[NumericBox](../../examples/compiler-numeric-fields/app.mc), which declares all
three numeric aliases and their nullable forms as instance fields. Constructor
writes, later scalar-class replacement, and compound/statement updates retain
the stored runtime numeric class. Uninitialized nullable fields read as Null;
a local snapshot remains usable after its field is cleared. Separate instances
retain independent values. SDK and generated applications match all
[25 console lines](../../examples/compiler-numeric-fields/expected.txt).
Structured export/reconstruction works for both applications with existing
field opcodes and DATA records. Generated fields use Null metadata slots and
source constructor writes; the compiler does not require the SDK's exact class
layout. Current contracts are in the [field specification](../specs/frontend/README.md#numeric-alias-fields).

For SDK reproduction, reuse the minimal-console manifest/resources with
`../../examples/compiler-numeric-fields` as the source path from
`scratch/compiler-numeric-fields/monkey.jungle`. The example contains its own
AppBase class. Use the specimen checker with `compiler-numeric-fields` as above;
current captures are in `scratch/check-numeric-fields/`.

## Container storage

SDK/compiler/simulator 9.2.0 / fr265 accept numeric aliases as Array elements
and Dictionary keys/values in the
[container example](../../examples/compiler-numeric-containers/app.mc).
Mixed Number/Long/Float/Double Arrays sort without conversion. Reads, indexed
updates, generic methods, nullable snapshots, and sized allocation preserve the
stored numeric classes. Dictionary `keys()` and `values()` expose those same
classes; lookup of an absent key returns Null.

A Dictionary constant containing `1`, `1l`, `1.0`, and `1.0d` has four entries,
with separately retrievable values. Equal mathematical value alone does not
merge keys of these four runtime classes on this target. The compiler retains
all scalar tags/bits rather than folding across classes. This does not establish
portable hash/equality behavior for arbitrary numeric edge cases.

SDK and generated applications match all
[69 console lines](../../examples/compiler-numeric-containers/expected.txt),
including ordered effects in computed Array and Dictionary literals. Both PRGs
support structured CODE/DATA export/reconstruction using existing scalar record
framing. See the [container contracts](../specs/frontend/README.md#numeric-alias-containers).
For SDK reproduction, reuse the minimal-console manifest/resources with
`../../examples/compiler-numeric-containers` as the source path from
`scratch/compiler-numeric-containers/monkey.jungle`; the example owns its AppBase
class. Use the specimen checker with `compiler-numeric-containers`; current
captures are in `scratch/check-numeric-containers-v4/`.

[lang]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang.html
[language]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/
