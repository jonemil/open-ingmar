# Retaining array stores

Date: 2026-10-14. Environment: SDK 9.2.0, `fr265`, debug, `-O 0`.
Question: how does opcode `0x49` participate in array-literal construction?

## Findings

**Experimentally observed:** an array literal with three effectful elements
allocates a length-three array and fills it in source order. The SDK uses
operandless `0x49` after each of the first two element calls, then ordinary
`aputv` after the third. No constant-array DATA record is required for this probe.

**Source-derived cross-check:** the pinned sources in
[instruction semantics](instruction-semantics.md#scope-and-sources) identify
`aputvdup`. Ghidra plugin `doc/opcodes.md`, entry `0x49`, and optimizer
`src/readprg/opcodes.ts`, `Aputvdup`, size and stack accounting, describe
`[..., array, index, value] → [..., array]`. No implementation code was copied.
The [instruction reference](../specs/vm/instructions.md#aputvdup) defines the
supported encoding and evidence limits.

Specimen: `scratch/array-fill-opcode/build/fr265/array-fill-opcode.prg`.
SHA-256:
`64a37f429efa16bd995b24fd7f797449b42c6cdc33e6a3d38b573a90aded09cf`.
The SDK debug XML places `onStart` at CODE+`0x40`:

```text
4a 02 01             argcincsp 0x02 0x01
38 03                ipush1 0x03
14                   newa
2e 00                dup 0x00
37                   ipushz
43 00 00 00 80       getselfv @element
2a                   frpush
38 0a                ipush1 0x0a
0f 02                invokem 0x02
49                   aputvdup
38 01                ipush1 0x01
# call element(20), then another aputvdup
# call element(30), then aputv and lputv 0x02
```

The retaining stores occur at CODE+`0x53` and `0x60`. The earlier `dup` leaves
a separate reference beneath the working array; the final consuming store
leaves that reference for local assignment. This interpretation agrees with
the source-derived stack model and successful execution.

SDK and reconstructed PRGs print `10`, `20`, `30`, `3`, `10`, `20`, `30`, one
per line: three element effects, array length, then stored values. CODE/DATA
export structurally, the complete PRG reconstructs byte-for-byte, and repeat
exports match. The compiler's current typed-append lowering remains unchanged;
direct filling belongs to the planned lowering review after coverage gaps close.

## Reproduction

Create ignored `scratch/array-fill-opcode/app.mc`:

```monkeyc
using Toybox.Application;
using Toybox.System;

function element(value) {
    System.println(value);
    return value;
}

class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        var values = [element(10), element(20), element(30)];
        System.println(values.size());
        System.println(values[0]);
        System.println(values[1]);
        System.println(values[2]);
        System.exit();
    }
}
```

Use the independently authored manifest/resources from
[Minimal Console](../../examples/minimal-console/README.md), with
`MinimalConsoleApp` as entry. Point the local Jungle file at that manifest,
this source directory, and those resources. Then:

```sh
sh scripts/build-example.sh scratch/array-fill-opcode
cargo run -p ciq-dis -- all scratch/array-fill-opcode/build/fr265/array-fill-opcode.prg scratch/array-fill-opcode/export
cargo run -p ciq-asm -- project scratch/array-fill-opcode/export scratch/array-fill-opcode/rebuilt.prg
cargo run -p ciq-dis -- all scratch/array-fill-opcode/rebuilt.prg scratch/array-fill-opcode/reexport
cmp scratch/array-fill-opcode/build/fr265/array-fill-opcode.prg scratch/array-fill-opcode/rebuilt.prg
diff -r scratch/array-fill-opcode/export scratch/array-fill-opcode/reexport
```

Run each PRG separately with
[the simulator helper](../../scripts/README.md); preserve session logs and check
the expected seven lines. Use fresh reconstruction output paths.

For the bounds probe, copy the exported project and change only the second
initializer's `ipush1 0x01` at CODE+`0x54` to `ipush1 0x03`. Reconstruct with
our own key using `-y keys/developer_key.der`. Only CODE operand byte `0x55`
and signing material differ. The simulator prints 10 and 20, then reports
`Array Out Of Bounds Error` at PC `0x10000061`; the helper exits 1. The third
element's effect and subsequent array inspection do not execute.

## Limits

Structured coverage is recorded in the [example audit](example-toolchain-audit.md).

The probe establishes encoding, element order, resulting contents, and the
observed bounds failure. It does not establish initialization visibility during
construction, other element/index types, invalid receivers, exception recovery,
or other SDK/device versions. Constant-array `apush` and its DATA records remain
separate [audited gaps](example-toolchain-audit.md).
