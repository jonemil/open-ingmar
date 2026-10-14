# Inversion opcode

Date: 2026-10-14. Environment: SDK 9.2.0, `fr265`, debug, `-O 0`.
Question: does audited opcode `0x2d` represent Boolean negation, Number
complement, or both?

## Findings

**Experimentally observed:** typed Number `return ~value` and Boolean
`return !value` compile to the same one-byte opcode `0x2d`, without operands.
The SDK debug XML maps the functions as follows:

```text
negate, CODE+0x04:     35 02 27 00 80 02 e6 30 13 00 12 01 2d 16
complement, CODE+0x32: 35 02 27 00 80 02 e6 30 13 00 12 01 2d 16
```

**Source-derived cross-check:** the pinned sources in
[instruction semantics](instruction-semantics.md#scope-and-sources) identify
`invv`, with no operands and a one-input/one-result stack effect. Relevant
locations: Ghidra plugin `doc/opcodes.md`, entry `0x2d`; optimizer
`src/readprg/opcodes.ts`, `Invv`, size and stack accounting. The plugin describes
bitwise inversion; the probe establishes Boolean negation as well. No external
implementation code was copied.

Specimen: `scratch/invert-opcode/build/fr265/invert-opcode.prg`. SHA-256:
`20bd6640e064a0d35ffe731d1f99187c84c670c348077b63e354f86dd5a23060`.

Both SDK and reconstructed programs print, one value per line:

```text
-1
0
-43
2147483647
-2147483648
false
true
```

The probe's CODE/DATA export structurally, its complete PRG reconstructs
byte-for-byte with preserved signing material, and its repeat export matches
the original export. The [instruction reference](../specs/instructions.md#invv)
defines the supported encoding and observed type-dependent behavior.

Our compiler already emits XOR with -1 for Number complement and comparison
with false for Boolean negation. Its semantic/text IR needs no change to
support disassembly/reassembly of this alternate SDK encoding. Choosing `invv`
in the compiler is a separate lowering decision.

## Reproduction

Create ignored `scratch/invert-opcode/app.mc`:

```monkeyc
using Toybox.Application;
using Toybox.System;

function complement(value as Toybox.Lang.Number) { return ~value; }
function negate(value as Toybox.Lang.Boolean) { return !value; }

class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(complement(0));
        System.println(complement(-1));
        System.println(complement(42));
        System.println(complement(-2147483648));
        System.println(complement(2147483647));
        System.println(negate(true));
        System.println(negate(false));
        System.exit();
    }
}
```

Use the independently authored manifest/resources from
[Minimal Console](../../examples/minimal-console/README.md), with
`MinimalConsoleApp` as entry. Point the local Jungle file at that manifest,
this source directory, and those resources. Then:

```sh
sh scripts/build-example.sh scratch/invert-opcode
cargo run -p ciq-dis -- all scratch/invert-opcode/build/fr265/invert-opcode.prg scratch/invert-opcode/export
cargo run -p ciq-asm -- project scratch/invert-opcode/export scratch/invert-opcode/rebuilt.prg
cargo run -p ciq-dis -- all scratch/invert-opcode/rebuilt.prg scratch/invert-opcode/reexport
cmp scratch/invert-opcode/build/fr265/invert-opcode.prg scratch/invert-opcode/rebuilt.prg
diff -r scratch/invert-opcode/export scratch/invert-opcode/reexport
```

Run each PRG separately with
[the simulator helper](../../scripts/README.md), checking the seven output
lines above. Preserve each session's logs. Use fresh reconstruction output paths.

## Limits

The repeated 56-example audit is saved at
`scratch/example-audit/inversion/report.json`. All 67 SDK PRGs and 48 generated
PRGs reconstruct byte-for-byte with identical repeat exports; documented
source/resumed-IR checks pass. SDK CODE is structured in 56 specimens, up from
49; DATA remains structured in 58. No `0x2d` gaps remain in the decoder walks.
Remaining opcode groups are `0x40` (nine examples) and `0x49` (six); DATA record
`0x03` remains in nine groups. The audit returns nonzero for 11 SDK coverage
issues. No additional opcode group was exposed in this run.

The experiment establishes Number/Boolean behavior in the selected environment.
Other value types, invalid operands, and other SDK/device versions remain
unverified. Remaining array-related encodings and DATA records are separate
[audited gaps](example-toolchain-audit.md).
