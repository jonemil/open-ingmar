# Long values, operators, and native methods

2026-10-16, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.

**Officially documented:** [Long][api] is a signed 64-bit integer available since
API 1.0.0. Its literals use an `l` suffix. Method declarations do not establish
our executable support for their results or numeric promotion rules.

**Observed:** the [authored specimen][example] accepts lower/uppercase suffixes,
zero, signed extrema, and values above Number's range. Module/instance calls,
local copies/reassignment, discarded calls, and console printing preserve the
values. SDK/generated consoles match the example's complete expected output.

Structured SDK CODE shows `lpush` (`0x31`) with an eight-byte big-endian operand
and `lpushz` (`0x3c`) for Long zero. The asymmetric literal `72623859790382856l`
has operand bytes `01 02 03 04 05 06 07 08`; negative values use two's complement.
These match the mnemonic/width descriptions in the pinned reference research
already recorded in [bootstrap provenance](rust-bootstrap.md).

The complete SDK/generated PRGs reconstruct through structured CODE/DATA and
stable re-export. No Long DATA record is required for these literal instructions.
Reproduce by building the example with the SDK, then using the
[console checker](../../scripts/README.md#console-specimen-check).
The ignored Jungle uses the shared native-names manifest/resources and the
tracked example's source path. Artifacts: `scratch/check-long-values/`.

## Arithmetic probe

**Observed:** Long/Long and Number/Long pairs execute the five arithmetic
operators with Long results. Typed SDK functions accept mixed addition and
subtraction as Long returns. Both operand orders retain values above 32 bits.
Negative division truncates toward zero; remainder follows the dividend.
Comparisons cover both matching and mixed operands. Compound Long locals accept
mixed numeric right operands; effect markers confirm left-to-right evaluation,
including an operand staged before a branching expression.

Maximum plus one and minimum minus one wrap across signed 64-bit boundaries,
including Number right operands. `4611686018427387904l * 4l` produces zero;
negating minimum produces minimum. These are SDK 9.2.0 / fr265 observations,
not evidence for every runtime or arithmetic boundary. The separate
[boundary probes](compiler-numeric-boundaries.md) cover minimum / -1, % -1,
and zero divisors. Source/storage narrowing is not inferred from
operator promotion. SDK/generated structured CODE/DATA round trips remain exact.
Artifacts: `scratch/check-long-arithmetic/`.

## Bitwise, shifts, and updates

**Observed (2026-10-17):** SDK 9.2.0 / fr265 accepts the mixed bitwise/shift
pairs as Long results. A typed Number-left/Long-count shift returns Long;
`1 << 32l` yields 4294967296. `!` and `~` both complement all 64 Long bits.
Signed right shifts extend the sign. Counts 0, 31, 32, 63, 64, 65, -1, -64,
and -65 agree with low-six-bit normalization. Long counts beyond Number's range
also agree: count 4294967296 acts as zero. These are simulator observations;
they do not establish every device implementation or arbitrary numeric types.

Statement increments/decrements wrap across both signed endpoints. Compound
bitwise/shift writes retain Long values, including a Long shift count. Markers
confirm receiver-free operand order in mixed shifts. SDK/generated consoles
match the full expected output, with structured reconstruction and stable
re-export. The VM already supplies these operations; no new encoding is needed.
Artifacts: `scratch/check-long-bits/`.

## Native methods and nullable conversions

**Observed (2026-10-17):** the grouped SDK 9.2.0 / fr265 specimen exercises
Long `abs`, `format`, `toLong`, `toNumber`, `toString`, `equals`, `compareTo`,
inherited `hashCode`/`weak`, Number `toLong`, and String
`toLong`/`toLongWithBase`. SDK/generated consoles match all 194 expected lines;
both programs reconstruct through structured sections and stable re-export.
Artifacts: `scratch/check-long-methods/`.

Long narrowing retains signed low 32 bits; Number widening sign-extends.
Minimum Long `abs` wraps to minimum. Equality distinguishes Long from equal
Number/Boolean/Char/String values in both receiver orders. Numeric comparisons
have the expected sign across these numeric classes and signed extrema.
Hash checks establish repeatability and equality consistency, not a fixed hash.
Effect markers confirm receiver-before-argument dispatch, including discarded
calls.

String conversions accept numeric prefixes (`"42tail"` gives 42, `"3.14"` gives
3), reject invalid prefixes and signed overflow with Null, and accept bases
0, 2, 16, and 36. Nullable parameters/results, copies, conditional joins, retry
loops, and direct guards retain Long values across saved IR and application
assembly. These observations do not establish arbitrary String parsing syntax
or all invalid-base behavior.

The public [Object weak contract][object] includes immutable Long among values
returned directly. The SDK statically types `Long.weak()` as
`WeakReference<Long>` and rejects chaining Long-only `toLong`/`toNumber` methods.
Using inherited `equals`/`toString` instead confirms the returned value behaves
as the original Long, including maximum Long. Our effective Long specialization
follows that runtime result; it does not claim the SDK's static type is Long.

SDK symbols identify `toLong` as `0x008000a9` and `toLongWithBase` as
`0x00800b1c`. Existing native dispatch instructions suffice; no opcode or DATA
record extension is required. Public method introductions are checked separately
in source and saved-IR tests, including Long's own `equals` since 1.3.0 rather
than the inherited Object 1.0.0 declaration.

Float/Double conversions and Long container/field storage remain outside this
probe. See the [current contract](../specs/frontend/numeric-values.md#long-values).

[object]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Object.html#weak-instance_function
[api]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Long.html
[example]: ../../examples/compiler-long-values/README.md
