# String/Char operators with wider numeric values

2026-10-17, SDK/compiler/simulator 9.2.0, `fr265`, debug, `-O 0`.
The [authored specimen][example] tests the numeric/String/Char intersections left
out of the earlier scalar groups. Public context: the [Monkey C reference][ref].
The exact operand restrictions below are observations, not inferred from native
`compareTo` signatures.

**Observed:** String concatenation accepts Long, Float, and Double in either
order, including nullable parameters and results. Null renders as `null`;
floating output retains the runtime's six-decimal rendering and signed zero.
String equality/inequality with these values, including nullable values, also
compiles. Distinct types compare unequal; these operators retain String identity
semantics and do not establish a non-null proof for numeric use.

Char + Long and Long + Char have Char results, verified with explicitly typed
SDK helpers. `'A' + 4294967297l` renders B, matching the low 32-bit offset in this
probe; no portable offset-range guarantee is inferred. Char equality and
inequality with Long/Float/Double compile and compare numeric values in both
orders. The SDK rejects their ordering comparisons, despite supporting the same
classes through native `compareTo`.

An isolated checking probe rejects Char/Float and Char/Double addition in both
orders, Char/Long subtraction in both orders, and Char-left Long multiplication,
division, and remainder. These rejected forms were not executed with checking
disabled. Preserve these restrictions rather than inserting conversions.

Char local/indexed `+= Long` and String local/indexed `+=` with wider numeric
values use captured-target read/combine/write semantics. Effect markers check
left-to-right operand evaluation and indexed RHS evaluation. String concatenation
with a conditional Double retains the earlier operand across the join.

Char `weak()` compares equal to the original Char and renders its character
through inherited `toString`. This agrees with the reference's immutable Char
rule. The effective result is Char, with API 1.3.0 required for the receiver;
ordinary Object/Array/Dictionary weak references remain separate. No weak call
is erased or replaced by a compiler copy.

## Reproduction and limits

Build the tracked example using an ignored Jungle selecting its source and the
shared native-names manifest/resources. Run the
[console checker](../../scripts/README.md#console-specimen-check) with that SDK
PRG. Both SDK/generated PRGs must reconstruct through structured CODE/DATA and
stable re-export, then match all 65 expected lines in the simulator.
Artifacts: `scratch/check-wide-operators/`. The checking-only rejection source
is `scratch/wide-operator-rejections/app.mc`; its expressions are listed above
so it can be recreated without retaining generated output.

Existing `addv`, `eq`, and `ne` instructions suffice. No conversion calls, new
opcodes, or DATA record types are needed. Nullability and result contracts are
recomputed in both source and saved IR. Other SDKs/devices, arbitrary Unicode
ranges, and floating Char arithmetic with checking disabled remain unverified.

[example]: ../../examples/compiler-wide-operators/README.md
[ref]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/
