# Example toolchain coverage audit

Baseline: 2026-10-14, SDK 9.2.0 / Forerunner 265. Scope: tracked example source commands, retained
SDK/generated PRGs, and the two Number barrel specimens. External repositories,
firmware, and unrelated scratch experiments are excluded. The complete coverage
baseline is recorded in `scratch/example-audit/constant-arrays/report.json`:
56 examples, 62 compiler commands, 67 SDK PRGs, and 48 generated applications.

## Findings

- All documented source/resumed-IR assembly paths agree; complete applications
  also produce identical unsigned PRGs through saved IR.
- SDK and generated CODE/DATA export structurally, rebuild byte-for-byte, and
  produce identical repeat exports. DEBUG, LINE_NUMBERS, EXCEPTIONS, and SYMBOLS
  also round-trip through their supported structured views.
- Both barrel specimens list, unpack, repack, and reopen with identical member
  contents. Compressed archive bytes need not match.

The review exposed these required format gaps, subsequently established by
focused specimens:

- `getv` (`0x0d`): [member reads](member-get-opcode.md).
- `isnull`/`isnotnull` (`0x20`/`0x4b`): [null checks](null-check-opcodes.md).
- `invv` (`0x2d`): [inversion](invert-opcode.md).
- `aputvdup` (`0x49`): [array filling and bounds failures](array-fill-opcode.md).
- `apush` (`0x40`) and DATA `0x03`: [constant arrays](constant-arrays.md).

Raw preservation had passed before those decoders were complete. That is why
[toolchain completion](../plan.md#toolchain-completion) requires structured
coverage separately from byte preservation. Per-example commands, hashes, gaps,
and exact comparisons belong in the audit report, not a duplicate Markdown
progress matrix.

## Frontend/project limits

SDK-only source probes expose existing scope limits; they are separate from
the decoder gaps in features already implemented:

- `hello-world` and `minimal-console` use bare function calls (`probe()` and
  `hello()`) that our frontend cannot resolve. Add unqualified call resolution
  to the core-language backlog, with global/current-receiver lookup rules.
- `module-names`, `multi-file`, and `barrel-app` stop at Graphics color constants.
  Their drawing/resource use and barrel dependency resolution also need the
  already planned SDK/project work. Each probe reports only its first failure.
- The Number barrel source compiles as a Rust compilation unit, but project
  resolution and production/consumption of compiler-generated barrels remain
  planned. Archive member preservation does not establish MIR interpretation.
- `compiler-probe/control-flow.mc` is an SDK research specimen containing
  try/catch and throw, rather than a documented supported frontend command.
  Its SDK instruction/section round trip passes; frontend exception lowering
  remains in the language backlog.

HEADER_VERSIONED, ENTRY_POINTS, IMPORTS, RESOURCES, PERMISSIONS, SETTINGS, and
DEVELOPER_SIGNATURE remain raw in all SDK section exports. These are existing
opaque export profiles, not newly discovered opcode failures. Some have
separate readers or bounded generation support; general structured editing
and opaque-reference relocation are not established by byte preservation.
Resource/Jungle/library work stays in its planned phase; signing remains the
existing own-key boundary.

## Procedure and limits

Use the [audit helper](../../scripts/README.md#example-toolchain-audit). It records
commands, hashes, diagnostics, exports, reconstruction, and saved-stage results.
Coverage fails for missing SDK specimens, raw CODE/DATA, or partial DATA with
opaque spans. Successful export alone is insufficient; simulator execution is
a separate check.

Retained SDK recipes are under `scratch/example-audit/sdk-builds/`; the explicit
index is `scratch/example-audit/sdk-index.json`. They select tracked sources and
local manifest/resources. AppBase classes use their declared entry; function-only
inputs need the recorded wrapper. Probe arguments match each function's arity;
Numbers helpers come from the compiler-unit example, and SDK print shorthand
uses an explicit `using Toybox.System`. See [shared reproduction](../research.md#shared-compiler-reproduction).
Not every retained input was rebuilt in the review; report hashes identify the
actual inspected binaries.

Opcode discovery starts at CODE zero and recorded source-line PCs, never guessed
widths or arbitrary-byte scans. Each walk stops at its first unknown instruction;
DATA walks likewise stop at an unknown record. Unknown inventories can therefore
be incomplete until earlier gaps are resolved.

This audit does not establish simulator behavior, safe length-changing edits,
other SDK/devices, or IQ profiles absent from the specimens. Use the focused
investigations for those observations and the specs for supported contracts.
