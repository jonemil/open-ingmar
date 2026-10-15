# Increment/decrement statements

Date: 2026-10-15. Environment: SDK 9.2.0, `fr265`, debug, `-O 0`.
Question: which update forms are accepted, and how do they read and write their
targets across source, IR, assembly, and runtime?

## Findings

**Officially documented:** the public [Monkey C reference][reference] lists
increment/decrement by one with prefix/postfix spellings. The reading map's
retained reference text contains these operator descriptions; the live page's
article body was unavailable through the browser text reader during this review.
The text alone does not establish C-style expression-result semantics.

**Experimentally observed:** `n++;`, `++n;`, `n--;`, and `--n;` compile and run
as statements for Number locals, explicit self fields, and Number-array elements.
Both spellings have identical observed behavior when their result is discarded.
SDK expression probes reject `return n++;` (extraneous `++` before `;`) and
`return ++n;` (missing `;` at `++`). A broader rejected probe includes arguments,
initializers, and loop conditions. Retained sources/logs are under
`scratch/compiler-increment-errors/` and
`scratch/compiler-increments/rejected-expressions.mc.txt`.

**Interpretation:** implement these forms as statements. The previous plan's
prefix/postfix expression results were an assumption, not an established source
contract. Other SDK versions have not been checked.

The [complete example][example] prints the [expected output][output] under both
SDK and generated simulator runs. It checks local/field/indexed updates, one
array/index call per target in that order, nullable Number guards, and loops.
Maximum Number increment wraps to minimum; minimum decrement wraps to maximum.
This establishes those two boundaries in the tested runtime, not arbitrary
promotion or failure behavior for other numeric classes.

The SDK also runs a Char increment probe: `'a'` becomes `b` with numeric value
98, and incrementing the first element of `['c']` produces `d`. It rejects
Char decrement with a Char/Number subtraction diagnostic. Our current update
profile is Number-only; [the plan](../plan.md#next-work) retains Char increment
as the next extension. Probe source/logs are in
`scratch/compiler-increment-errors/char-increment-*` and `char-decrement.mc.txt`.

## Lowering and reconstruction

SDK local update at CODE+`0x68` uses
`12 02 38 01 03 13 02`: load local 2, push one, add, store local 2.
Decrement substitutes `subv` (`0x04`). Field updates capture self/member and use
`getv`/`putv`; array updates capture array/index and use `agetv`/`aputv`.
All required instructions were already supported. No new opcode or DATA record
was encountered in this specimen.

Our source builder retains a Number `Step` between the existing target read and
write. Saved IR uses `.increment`/`.decrement`; validation checks non-null Number
operands, single use, source order, and write-back contracts. Both saved and
native paths select push-one plus `addv`/`subv`. Indexed updates reuse compound
assignment's captured location, including its frame budget.

Source/resumed unit IR produce identical unsigned PRGs. SDK and generated PRGs
export CODE/DATA structurally, reconstruct byte-for-byte, and produce identical
repeat exports. Own-key signing is checked separately from reconstruction and
simulator behavior. Artifacts, console logs, and exports are under
`scratch/compiler-increments/`. Artifact SHA-256 values:

- Generated unsigned:
  `de9cfdf007e27d1e7f69597269de0bda03c4520eb7d7e09858d2e8a580dbded8`.
- SDK:
  `63e87de199a30e6e023a6b033e80aeeba6b716aa3fb479090dd1178a82aea0ea`.

The repeated audit at `scratch/example-audit/increments/report.json` covers all
57 examples with no coverage issues, including this example's SDK/generated
structured reconstruction and saved-stage equivalence.

## Reproduction and a separate SDK failure

Use the example's compile/sign/run commands with unused output paths. For SDK
comparison, create `scratch/compiler-increments/monkey.jungle`:

```text
project.manifest=../native-names/manifest.xml
base.sourcePath=../../examples/compiler-increments
base.resourcePath=../native-names/resources
```

Reuse the ignored Minimal Console manifest/resource scaffold with
`MinimalConsoleApp` as entry. Build and export:

```sh
sh scripts/build-example.sh scratch/compiler-increments
cargo run -p ciq-dis -- all scratch/compiler-increments/build/fr265/compiler-increments.prg scratch/compiler-increments/sdk-export
cargo run -p ciq-asm -- project scratch/compiler-increments/sdk-export scratch/compiler-increments/sdk-rebuilt.prg
cargo run -p ciq-dis -- all scratch/compiler-increments/sdk-rebuilt.prg scratch/compiler-increments/sdk-reexport
```

Run the SDK PRG separately and compare its console log to `expected.txt`. Repeat
export/reconstruction for the generated PRG, comparing input/rebuilt bytes and
both exported trees.

Adding `a[(true || false).hashCode() - 1]++;` before exit makes SDK 9.2.0 report
an internal compiler error. Removing that update permits compilation, including
the nullable and boundary checks. Logs are `sdk-full-build.log`,
`sdk-without-nullable-build.log`, and `sdk-boundaries-build.log`. Our source/IR
checks accept the typed branching index and retain single target capture; this
specific SDK failure provides no runtime comparison. Do not infer a different
source evaluation contract from the compiler error.

[reference]: https://developer.garmin.com/connect-iq/reference-guides/monkey-c-reference/
[example]: ../../examples/compiler-increments/README.md
[output]: ../../examples/compiler-increments/expected.txt
