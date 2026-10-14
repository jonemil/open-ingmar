# Stacked member get

Date: 2026-10-14. Environment: SDK 9.2.0, `fr265`, debug, `-O 0`.
Question: what does opcode `0x0d` in the compound-target specimen encode?

## Findings

**Experimentally observed:** `self.count += self.value()` emits a one-byte
`0x0d` after pushing and duplicating the receiver/member-symbol pair. The next
instruction loads the RHS callable; no immediate operand follows `0x0d`.
The same opcode appears in `self.count *= 2`.

**Source-derived cross-check:** the pinned references in
[instruction semantics](instruction-semantics.md#scope-and-sources) name this
instruction `getv`. Ghidra plugin `doc/opcodes.md`, entry `0x0d`, and optimizer
`src/readprg/opcodes.ts`, `Getv` and stack accounting, describe consumption of
receiver and member symbol and production of a member value. No implementation
code was copied. The [instruction reference](../specs/instructions.md#getv)
records the supported encoding and interpretation.

Specimen: `scratch/member-get-opcode/build/fr265/member-get-opcode.prg`.
SHA-256:
`e08948466f9814dfe9f84108027ec372ef0a936933443873c8b8aa5e8e6471c8`.
The SDK debug XML places `onStart` at CODE+`0x24`.
Its compound-add sequence begins at CODE+`0x38`:

```text
44                    getself
27 00 80 04 da        spush @count
2e 01                 dup 0x01
2e 01                 dup 0x01
0d                    getv
43 00 80 01 53        getselfv @value
48                    invokemz
03                    addv
0e                    putv
```

`getv` is at CODE+`0x42`; the multiply uses another at CODE+`0x70`.
**Interpretation:** duplicating the receiver/symbol retains the eventual store
location while `getv` reads the old count. The RHS changes count from 10 to 20
and returns 3; the final result is 13. Both SDK and reconstructed PRGs print
`10`, `field rhs`, `13`, `20`, one per line.

Our tools export this probe's CODE/DATA structurally, reconstruct the complete
PRG byte-for-byte without re-signing, and produce identical repeat exports.
The compiler already lowers self-field reads using immediate-symbol `getselfv`
and preserves compound-update order; no new IR or compiler lowering is required
for accepting the SDK's alternative sequence.

## Reproduction

Create ignored `scratch/member-get-opcode/app.mc`:

```monkeyc
using Toybox.Application;
using Toybox.System;

class MinimalConsoleApp extends Application.AppBase {
    var count = 10;
    function initialize() { AppBase.initialize(); }
    function value() {
        System.println("field rhs");
        self.count = 20;
        return 3;
    }
    function onStart(state) {
        System.println(self.count);
        self.count += self.value();
        System.println(self.count);
        self.count = 10;
        self.count *= 2;
        System.println(self.count);
        System.exit();
    }
}
```

Use the independently authored manifest/resources from
[Minimal Console](../../examples/minimal-console/README.md), with
`MinimalConsoleApp` as entry. Point the local Jungle file at that manifest,
this source directory, and those resources. Then:

```sh
sh scripts/build-example.sh scratch/member-get-opcode
cargo run -p ciq-dis -- all scratch/member-get-opcode/build/fr265/member-get-opcode.prg scratch/member-get-opcode/export
cargo run -p ciq-asm -- project scratch/member-get-opcode/export scratch/member-get-opcode/rebuilt.prg
cargo run -p ciq-dis -- all scratch/member-get-opcode/rebuilt.prg scratch/member-get-opcode/reexport
cmp scratch/member-get-opcode/build/fr265/member-get-opcode.prg scratch/member-get-opcode/rebuilt.prg
diff -r scratch/member-get-opcode/export scratch/member-get-opcode/reexport
```

Run each PRG separately with
[the simulator helper](../../scripts/README.md) and check the four output lines.
Save logs before the next session. Use fresh export/output paths when repeating
reconstruction.

## Limits

The repeated 56-example audit is saved at
`scratch/example-audit/member-get/report.json`. All 67 SDK PRGs and 48 generated
PRGs reconstruct byte-for-byte with identical repeat exports; documented
source/resumed-IR checks pass. No `0x0d` gap remains in the decoder walks.
SDK CODE is structured in 49 specimens and DATA in 58. Remaining opcode groups
are `0x2d` (11 examples), `0x40` (nine), and `0x49` (six); DATA record `0x03`
remains in nine groups. The audit returns nonzero for 18 SDK coverage issues.

The probe establishes the field-read encoding and compound-update behavior.
It does not characterize inheritance, missing/read-only members, invalid
receivers, or other SDK/device versions. The larger compound-target specimen
also contains unrelated audited opcodes and DATA records; adding `getv` alone
does not establish complete structured coverage for that program.
