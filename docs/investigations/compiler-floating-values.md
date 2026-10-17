# Float/Double values and native methods

2026-10-17, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.

**Officially documented:** [Float][float] and [Double][double] are 32/64-bit
floating values since API 1.0.0. Decimal literals default to Float; Double uses
`d`. Their API pages describe numeric conversions and formatting; `compareTo`
requires 5.0.0. Double declares `equals` since 1.3.0, while Float inherits Object's
1.0.0 method. [String][string] declares nullable floating conversions.

**Observed:** the [authored specimen][example] accepts decimal/exponent Float
literals, lowercase Double suffixes including `1d`, and `1e2`. Uppercase `D`
was rejected in both `1D` and `1.0D`; our source parser follows that restriction.
Float rounds 16777217 to 16777216; Double retains 16777217. Both signed-zero
literal signs survive calls/printing. Runtime negation of positive zero gives
positive zero, unlike a negative-zero literal. SDK CODE uses same-type zero
subtraction for runtime negation.

Arithmetic accepts Number/Float pairs as Float and Long/Float pairs as Double;
a typed SDK helper checks the latter result. Double dominates other numeric
operands. The four arithmetic operators and six comparisons execute directly;
SDK floating `%` and `%=` are rejected. Compound writes and statement updates
retain floating values; markers check receiver/operand order. These tests do not
establish non-finite arithmetic or out-of-range conversion behavior.

The native group covers `abs`, `format`, `toString`, all four numeric conversions,
`equals`, `compareTo`, inherited `hashCode`, and immutable `weak`. Numeric
conversions truncate in-range negative fractions toward zero, round Long to
Float precision, and retain the wider Long in Double. Equality distinguishes
Float from Double and integer classes; numeric comparisons cross these classes.
Weak-result equality checks agree with immutable value behavior, without
claiming the SDK's static WeakReference signature is the effective result type.

A preliminary probe compared equal, separately created Double values: `equals`
returned true but their inherited hashes differed. The retained specimen checks
repeatability on the same Double instead; no cross-instance hash guarantee is
inferred. Float equal-value hashes matched in this probe. Hash values remain
runtime supplied and must not be compiled into expectations.

String parsing accepts decimal exponents and numeric prefixes, returns Null for
`"bad"`, and propagates through guarded locals, copies, nullable parameters,
results, loops, and conditional joins. Arbitrary parsing syntax, overflow, and
non-finite inputs remain unverified.

## Instruction and symbol evidence

SDK CODE contains `fpush` (`0x26`, four operand bytes), `dpush` (`0x32`, eight),
`fpushz` (`0x3b`), and `dpushz` (`0x3d`). `1.25` is `3f a0 00 00`;
`-5.75d` is `c0 17 00 00 00 00 00 00`. Negative zero uses full-width pushes
with only the sign bit set. Positive zero uses the operand-free forms. These
support big-endian IEEE 754 interpretation. No DATA extension is needed for
these immediates. SDK symbols bind `toFloat` to `0x008000a5` and `toDouble` to
`0x008000a8`; existing native invocation instructions suffice.

Reproduce by compiling the tracked example with a Jungle pointing at its source
and the shared ignored native-names manifest/resources, then using the
[console checker](../../scripts/README.md#console-specimen-check). SDK/generated
PRGs must reconstruct through structured CODE/DATA and stable re-export, and
both consoles must match all 111 tracked expected lines. Artifacts:
`scratch/check-float-values/`. Use a PRG snapshot without a neighboring SDK debug
XML for the runtime comparison; uploading that optional file delayed the first
probe beyond its observation window.

[float]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Float.html
[double]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Double.html
[string]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/String.html
[example]: ../../examples/compiler-float-values/README.md
