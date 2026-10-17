# Long literals, value transport, and arithmetic

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
not evidence for every runtime or arithmetic boundary. Minimum / -1, % -1, and
zero divisors were not run. Source/storage narrowing is not inferred from
operator promotion. SDK/generated structured CODE/DATA round trips remain exact.
Artifacts: `scratch/check-long-arithmetic/`.

Bitwise operations, shifts, updates, conversions, nullable/container values, and
native Long methods were not established by this specimen. See the
[current contract](../specs/compiler-subset.md#long-values) for accepted scope.

[api]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/Long.html
[example]: ../../examples/compiler-long-values/README.md
