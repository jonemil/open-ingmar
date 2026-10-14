# Example toolchain coverage audit

Reviewed 2026-10-14 at Git revision `ffb7b2685256891f01a8cd71b02aec4d53cf733c`,
using SDK 9.2.0 / Forerunner 265 specimens and current workspace tools.
The scope is all 56 project directories under `examples/`, including the
13 source files in `compiler-probe`. External repositories, firmware, and
unrelated scratch experiments are excluded.

## Findings

- All 62 documented compiler source commands pass. Source/resumed IR assembly
  matches for all of them; all 48 complete-app commands also produce identical
  unsigned PRGs through saved IR.
- All 67 SDK PRGs (55 example groups) export and reconstruct byte-for-byte.
  CODE is structured in 40 specimens and raw in 27. DATA is structured in 58
  specimens and raw in nine. Preservation succeeds despite these decoding gaps.
- Both retained Number barrel archives list, unpack, repack, and reopen with
  identical member contents. Their outer archive bytes need not be identical.
- Every reviewed PRG exports DEBUG, LINE_NUMBERS, EXCEPTIONS, and SYMBOLS
  structurally. No additional parsing/reconstruction failures were observed.

The five opcode bytes beyond the previously identified `0x0d` were additional
toolchain gaps at this audit. The
[null-check follow-up](null-check-opcodes.md) establishes `0x20`/`0x4b` and
records the subsequent reconstruction checks; the table below records the
initial inventory.

| Unsupported opcode | Example groups | Representative |
|---|---|---|
| `0x0d` | 1 | [compiler-compound-targets][e17] |
| `0x20` | 17 | [compiler-array-reads][e06] |
| `0x2d` | 5 | [compiler-boolean-negation][e10] |
| `0x40` | 9 | [compiler-array-literals][e04] |
| `0x49` | 6 | [compiler-array-literals][e04] |
| `0x4b` | 5 | [compiler-array-writes][e08] |

The DATA walker stops on record tag `0x03` in nine example groups:
`compiler-array-literals`, `compiler-array-writes`, `compiler-compound-locals`,
`compiler-compound-targets`, `compiler-number-array-literals`,
`compiler-number-bits`, `compiler-number-literals`, `compiler-number-plus`,
and `compiler-typed-array-writes`. This is the same unresolved record category
as the compound-target finding, now established across more examples.

## Coverage by example

Source counts refer to documented commands; SDK-only projects are probed
separately below. CODE/DATA columns count structured SDK exports over the
number of reviewed SDK specimens. All listed PRGs preserve bytes on rebuild.

| Example | Source commands | SDK CODE | SDK DATA |
|---|---|---|---|
| [barrel-app][e01] | 0 | 1/1 | 1/1 |
| [compiler-application][e02] | 1 | 1/1 | 1/1 |
| [compiler-array-add][e03] | 1 | 1/1 | 1/1 |
| [compiler-array-literals][e04] | 1 | 0/1 | 0/1 |
| [compiler-array-mutation][e05] | 1 | 1/1 | 1/1 |
| [compiler-array-reads][e06] | 1 | 0/1 | 1/1 |
| [compiler-array-transforms][e07] | 1 | 1/1 | 1/1 |
| [compiler-array-writes][e08] | 1 | 0/1 | 0/1 |
| [compiler-boolean-methods][e09] | 1 | 1/1 | 1/1 |
| [compiler-boolean-negation][e10] | 1 | 0/1 | 1/1 |
| [compiler-char-arrays][e11] | 1 | 0/1 | 1/1 |
| [compiler-char-literals][e12] | 1 | 1/1 | 1/1 |
| [compiler-char-methods][e13] | 1 | 0/1 | 1/1 |
| [compiler-char-operators][e14] | 1 | 0/1 | 1/1 |
| [compiler-class][e15] | 1 | 1/1 | 1/1 |
| [compiler-compound-locals][e16] | 1 | 0/1 | 0/1 |
| [compiler-compound-targets][e17] | 1 | 0/1 | 0/1 |
| [compiler-empty-arrays][e18] | 1 | 1/1 | 1/1 |
| [compiler-fields][e19] | 1 | 1/1 | 1/1 |
| [compiler-immutable-weak][e20] | 1 | 1/1 | 1/1 |
| [compiler-initial-view][e21] | 1 | 1/1 | 1/1 |
| [compiler-input-counter][e22] | 1 | 0/1 | 1/1 |
| [compiler-lang-values][e23] | 1 | 1/1 | 1/1 |
| [compiler-lifecycle][e24] | 1 | 1/1 | 1/1 |
| [compiler-logical-values][e25] | 1 | 0/1 | 1/1 |
| [compiler-native-calls][e26] | 1 | 1/1 | 1/1 |
| [compiler-native-statements][e27] | 1 | 0/1 | 1/1 |
| [compiler-native-views][e28] | 1 | 0/1 | 1/1 |
| [compiler-nullable-contracts][e29] | 1 | 0/1 | 1/1 |
| [compiler-nullable-numbers][e30] | 1 | 0/1 | 1/1 |
| [compiler-number-array-literals][e31] | 1 | 0/1 | 0/1 |
| [compiler-number-arrays][e32] | 1 | 0/1 | 1/1 |
| [compiler-number-bits][e33] | 1 | 0/1 | 0/1 |
| [compiler-number-comparisons][e34] | 1 | 0/1 | 1/1 |
| [compiler-number-literals][e35] | 1 | 0/1 | 0/1 |
| [compiler-number-methods][e36] | 1 | 1/1 | 1/1 |
| [compiler-number-negation][e37] | 1 | 0/1 | 1/1 |
| [compiler-number-plus][e38] | 1 | 0/1 | 0/1 |
| [compiler-number-products][e39] | 1 | 1/1 | 1/1 |
| [compiler-object-calls][e40] | 1 | 1/1 | 1/1 |
| [compiler-probe][e41] | 12 | 13/13 | 13/13 |
| [compiler-reference-fields][e42] | 1 | 1/1 | 1/1 |
| [compiler-short-circuit][e43] | 1 | 0/1 | 1/1 |
| [compiler-source-signatures][e44] | 1 | 1/1 | 1/1 |
| [compiler-string-methods][e45] | 1 | 0/1 | 1/1 |
| [compiler-string-operators][e46] | 1 | 0/1 | 1/1 |
| [compiler-substring][e47] | 1 | 0/1 | 1/1 |
| [compiler-typed-array-writes][e48] | 1 | 0/1 | 0/1 |
| [compiler-unit][e49] | 2 | 1/1 | 1/1 |
| [compiler-visible-view][e50] | 1 | 1/1 | 1/1 |
| [compiler-void][e51] | 1 | 1/1 | 1/1 |
| [hello-world][e52] | 0 | 1/1 | 1/1 |
| [minimal-console][e53] | 0 | 1/1 | 1/1 |
| [module-names][e54] | 0 | 1/1 | 1/1 |
| [multi-file][e55] | 0 | 1/1 | 1/1 |
| [number-barrel][e56] | 0 | barrel | barrel |

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

Run the [audit helper](../../scripts/audit-examples.py) from the workspace root:

```sh
cargo build -p ciq-compiler -p ciq-dis -p ciq-asm -p ciq-package
python3 scripts/audit-examples.py --output scratch/example-audit/new-run
```

The helper records commands, artifact SHA-256 hashes, diagnostics, section
exports, reconstruction results, and source stop/resume checks in `report.json`
and per-example logs. Its nonzero coverage status deliberately includes raw
CODE/DATA fallback and missing SDK specimens; successful export alone is not
a passing audit. It does not run simulator sessions.

This audit reused 47 retained SDK builds and rebuilt 20 missing SDK specimens.
For missing compiler examples, select the tracked sources through an ignored
Jungle and the existing manifest/resources scaffold. Use the declared AppBase
class as manifest entry when present. Otherwise add a minimal AppBase wrapper
whose onStart calls the documented global entry. Probe functions receive zero-valued
arguments matching their arity; Numbers helpers come from the tracked
compiler-unit source. An SDK-only `using Toybox.System` prefix supplies the
global print shorthand used by our frontend examples.

The ignored build recipes are under `scratch/example-audit/sdk-builds/`.
A JSON example-name to PRG-path mapping (or list of paths for compiler-probe)
selects them with `--sdk-index scratch/example-audit/sdk-index.json`.
The completed report is `scratch/example-audit/final/report.json`.
Retained builds were not all rebuilt against current source, so recorded
artifact hashes identify exactly what was inspected.

Opcode discovery starts at CODE offset zero and exported source-line PCs,
then lets the current decoder walk until its first unsupported instruction.
It never guesses widths or scans arbitrary bytes for apparent opcodes. The
inventory of six opcode bytes is a lower bound: unknown instructions can hide
further gaps between recorded PCs. DATA discovery likewise stops at its first unknown
record. Repeat the audit after each decoding group is added.

This review does not retest simulator execution, length-changing edits, other
SDK/device versions, or IQ archives absent from the example directories.
Existing simulator evidence remains separate. Fix the demonstrated decoding
and DATA gaps, rerun structured reconstruction checks, then resume language
implementation under the [completion rule](../plan.md#toolchain-completion).

[e01]: ../../examples/barrel-app/README.md
[e02]: ../../examples/compiler-application/README.md
[e03]: ../../examples/compiler-array-add/README.md
[e04]: ../../examples/compiler-array-literals/README.md
[e05]: ../../examples/compiler-array-mutation/README.md
[e06]: ../../examples/compiler-array-reads/README.md
[e07]: ../../examples/compiler-array-transforms/README.md
[e08]: ../../examples/compiler-array-writes/README.md
[e09]: ../../examples/compiler-boolean-methods/README.md
[e10]: ../../examples/compiler-boolean-negation/README.md
[e11]: ../../examples/compiler-char-arrays/README.md
[e12]: ../../examples/compiler-char-literals/README.md
[e13]: ../../examples/compiler-char-methods/README.md
[e14]: ../../examples/compiler-char-operators/README.md
[e15]: ../../examples/compiler-class/README.md
[e16]: ../../examples/compiler-compound-locals/README.md
[e17]: ../../examples/compiler-compound-targets/README.md
[e18]: ../../examples/compiler-empty-arrays/README.md
[e19]: ../../examples/compiler-fields/README.md
[e20]: ../../examples/compiler-immutable-weak/README.md
[e21]: ../../examples/compiler-initial-view/README.md
[e22]: ../../examples/compiler-input-counter/README.md
[e23]: ../../examples/compiler-lang-values/README.md
[e24]: ../../examples/compiler-lifecycle/README.md
[e25]: ../../examples/compiler-logical-values/README.md
[e26]: ../../examples/compiler-native-calls/README.md
[e27]: ../../examples/compiler-native-statements/README.md
[e28]: ../../examples/compiler-native-views/README.md
[e29]: ../../examples/compiler-nullable-contracts/README.md
[e30]: ../../examples/compiler-nullable-numbers/README.md
[e31]: ../../examples/compiler-number-array-literals/README.md
[e32]: ../../examples/compiler-number-arrays/README.md
[e33]: ../../examples/compiler-number-bits/README.md
[e34]: ../../examples/compiler-number-comparisons/README.md
[e35]: ../../examples/compiler-number-literals/README.md
[e36]: ../../examples/compiler-number-methods/README.md
[e37]: ../../examples/compiler-number-negation/README.md
[e38]: ../../examples/compiler-number-plus/README.md
[e39]: ../../examples/compiler-number-products/README.md
[e40]: ../../examples/compiler-object-calls/README.md
[e41]: ../../examples/compiler-probe/README.md
[e42]: ../../examples/compiler-reference-fields/README.md
[e43]: ../../examples/compiler-short-circuit/README.md
[e44]: ../../examples/compiler-source-signatures/README.md
[e45]: ../../examples/compiler-string-methods/README.md
[e46]: ../../examples/compiler-string-operators/README.md
[e47]: ../../examples/compiler-substring/README.md
[e48]: ../../examples/compiler-typed-array-writes/README.md
[e49]: ../../examples/compiler-unit/README.md
[e50]: ../../examples/compiler-visible-view/README.md
[e51]: ../../examples/compiler-void/README.md
[e52]: ../../examples/hello-world/README.md
[e53]: ../../examples/minimal-console/README.md
[e54]: ../../examples/module-names/README.md
[e55]: ../../examples/multi-file/README.md
[e56]: ../../examples/number-barrel/README.md
