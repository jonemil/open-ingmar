# Null-check opcodes

Date: 2026-10-14. Environment: SDK 9.2.0, `fr265`, debug, `-O 0`.
Question: what do audited opcode bytes `0x20` and `0x4b` encode, and can our
assembler/disassembler reconstruct programs containing them structurally?

## Findings

**Experimentally observed:** the SDK emits `0x20` for `value == null` and
`0x4b` for `value != null`. Both have no operands. The focused functions load
one argument, execute the check, and return a Boolean. The results distinguish
null from Number zero, Boolean false, and an empty String.

**Source-derived cross-check:** the pinned sources in
[instruction semantics](instruction-semantics.md#scope-and-sources) name these
instructions `isnull` and `isnotnull` and describe a one-input/one-result stack
effect. Relevant locations: Ghidra plugin `doc/opcodes.md` entries `0x20` and
`0x4b`; optimizer `src/readprg/opcodes.ts` opcode enum and stack accounting.
No external implementation code was copied.

Specimen: `scratch/null-check-opcodes/build/fr265/null-check-opcodes.prg`.
SHA-256:
`f3b521261dea1f44ad4c3ed0ecde8b1ca7c534f7430f2ff64a37fcf4e0936dd9`.

The SDK's debug XML maps these complete functions to CODE+`0x24` and `0x32`:

```text
isNull:    35 02 27 00 80 02 e6 30 13 00 12 01 20 16
isPresent: 35 02 27 00 80 02 e6 30 13 00 12 01 4b 16
```

`0x20` and `0x4b` are immediately followed by `return`. Both SDK and rebuilt
programs print `true false false false false true true true`, one per line.
The complete PRG reconstructs byte-for-byte, with structured CODE/DATA and
identical first and second exports. Signing material is preserved for this
unchanged reconstruction; no original-key access is needed.

The compiler already lowers null comparisons through supported comparison
instructions. No new semantic IR operation or instruction-selection policy is
required to support these SDK encodings in assembly/disassembly. The
[instruction reference](../specs/instructions.md#isnull) defines their encoding.

## Reproduction

Create ignored `scratch/null-check-opcodes/app.mc`:

```monkeyc
using Toybox.Application;
using Toybox.System;

function isNull(value) { return value == null; }
function isPresent(value) { return value != null; }

class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        System.println(isNull(null));
        System.println(isNull(0));
        System.println(isNull(false));
        System.println(isNull(""));
        System.println(isPresent(null));
        System.println(isPresent(0));
        System.println(isPresent(false));
        System.println(isPresent(""));
        System.exit();
    }
}
```

Use the independently authored manifest/resources from
[Minimal Console](../../examples/minimal-console/README.md), with
`MinimalConsoleApp` as entry. Point a local Jungle file at that manifest,
this source directory, and those resources. Then:

```sh
sh scripts/build-example.sh scratch/null-check-opcodes
cargo run -p ciq-dis -- all scratch/null-check-opcodes/build/fr265/null-check-opcodes.prg scratch/null-check-opcodes/export
cargo run -p ciq-asm -- project scratch/null-check-opcodes/export scratch/null-check-opcodes/rebuilt.prg
cargo run -p ciq-dis -- all scratch/null-check-opcodes/rebuilt.prg scratch/null-check-opcodes/reexport
cmp scratch/null-check-opcodes/build/fr265/null-check-opcodes.prg scratch/null-check-opcodes/rebuilt.prg
diff -r scratch/null-check-opcodes/export scratch/null-check-opcodes/reexport
```

Run each PRG separately through
[the simulator helper](../../scripts/README.md); compare console output with
the expected Boolean sequence. Save captures/logs before the next session.
Rebuilt artifacts require fresh export/output paths.

## Limits

Structured coverage is recorded in the [example audit](example-toolchain-audit.md).

These observations establish encoding and null-test results for the selected
values and environment. They do not establish physical VM stack representation
or compatibility with other SDK/device versions. Broader unknown opcodes and
DATA records remain separate [audited gaps](example-toolchain-audit.md).
