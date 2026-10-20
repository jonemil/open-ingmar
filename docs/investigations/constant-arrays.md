# Constant-array records and loads

Date: 2026-10-14. Environment: SDK 9.2.0, `fr265`, debug, `-O 0`.
Question: how do audited opcode `0x40` and DATA record `0x03` relate, and can
their contents and references be represented as editable assembly?

## Findings

**Experimentally observed:** `apush` (`0x40`) has a four-byte big-endian DATA
address pointing at the outer `0x03` record tag. The supported specimens contain
flat arrays with typed Number, Char, null, and Boolean elements. The serialized
root has magic `0xda7ada7a` and type `0x05`.

**Source-derived cross-check:** the pinned Ghidra plugin in
[instruction semantics](instruction-semantics.md#scope-and-sources) names
`apush` in `doc/opcodes.md` and describes container framing in `doc/prg.md`,
“JSON Container.” The optimizer's pinned `src/readprg/opcodes.ts` independently
identifies its four-byte operand. No implementation code was copied.

**Framing observation:** the outer and inner lengths both exclude the final zero
terminator. The outer includes magic/inner-length fields; the inner starts with
the root type/count. This distinction is necessary for walking subsequent
records and is explicit in [the local format spec](../specs/formats/data-arrays.md).
The external container description alone was insufficient to fix the boundaries.

Focused specimen: `scratch/constant-array/build/fr265/constant-array.prg`.
SHA-256:
`0204712cd3e6e27f286a2eb56df726476271dcfaee6c1965dfd405e3a762a569`.

- DATA+`0x329b`: two Char values, raw bits `0x61` and `0x62`.
- DATA+`0x32b8`: true/false/null, element bytes `09 01 09 00 00`.
- DATA+`0x32d0`: Numbers 10/20/30. Outer length 28, inner length 20, total
  record size 34. CODE+`0x2e` uses `40 00 00 32 d0` in `make()`.

The Number record's bytes are:

```text
03 00 00 00 1c da 7a da 7a 00 00 00 14 05 00 00 00 03
01 00 00 00 0a 01 00 00 00 14 01 00 00 00 1e 00
```

SDK and reconstructed programs print `99`, `10`, `b`, `true`, `false`, `null`,
one per line. Both calls to `make()` execute the same `apush`, but changing the
first array leaves the second unchanged. **Interpretation:** loads create
independently mutable arrays for this flat scalar profile. Deep-copy behavior
for nested/container elements is not established.

CODE/DATA export structurally and reconstruct byte-for-byte with identical
repeat exports. Editing `.element number=0x0000000a` to `number=0x0000000c`
and reassembling with our key changes the second printed value from 10 to 12.
The first remains 99, and Char/Boolean/null results remain unchanged.

## Reproduction

Create ignored `scratch/constant-array/app.mc`:

```monkeyc
using Toybox.Application;
using Toybox.System;
function make() { return [10, 20, 30]; }
class MinimalConsoleApp extends Application.AppBase {
    function initialize() { AppBase.initialize(); }
    function onStart(state) {
        var first = make();
        var second = make();
        first[0] = 99;
        System.println(first[0]);
        System.println(second[0]);
        var chars = ['a', 'b'];
        System.println(chars[1]);
        var flags = [true, false, null];
        System.println(flags[0]);
        System.println(flags[1]);
        System.println(flags[2]);
        System.exit();
    }
}
```

Reuse our [Minimal Console](../../examples/minimal-console/README.md)
manifest/resources with `MinimalConsoleApp` as entry. Point the local Jungle
file at that manifest, this source directory, and those resources:

```sh
sh scripts/build-example.sh scratch/constant-array
cargo run -p ciq-dis -- all scratch/constant-array/build/fr265/constant-array.prg scratch/constant-array/export
cargo run -p ciq-asm -- project scratch/constant-array/export scratch/constant-array/rebuilt.prg
cargo run -p ciq-dis -- all scratch/constant-array/rebuilt.prg scratch/constant-array/reexport
cmp scratch/constant-array/build/fr265/constant-array.prg scratch/constant-array/rebuilt.prg
diff -r scratch/constant-array/export scratch/constant-array/reexport
```

Run SDK/rebuilt PRGs separately with
[the simulator helper](../../scripts/README.md), preserving logs before the next
session. For the value edit, copy the project, change the Number element in DATA,
and reconstruct with `-y keys/developer_key.der`. Use fresh output paths.

## Coverage and limits

The repeated [example audit](example-toolchain-audit.md) is saved at
`scratch/example-audit/constant-arrays/report.json`. All 67 SDK and 48 generated
PRGs reconstruct byte-for-byte with identical repeat exports. All 115 export
CODE/DATA structurally, with no opaque `.bytes` spans in DATA. Source/resumed-IR
checks and both barrel member round trips pass. No further opcode or DATA-record
gap is exposed in these specimens; this does not establish complete Garmin
format compatibility.

The helper now treats partial DATA exports containing opaque spans as incomplete
coverage even though their export encoding is `data`. Unknown container forms
continue to use raw preservation and cannot satisfy structured coverage.

String pools, nested arrays, Dictionary/ByteArray, other scalar types, class-field
array-reference relocation, and other SDK/device versions remain unverified or
unsupported. Compiler selection and private construction are covered by the
[lowering review](natural-lowering.md#constant-array-selection).

## String pools

On 2026-10-16, SDK 9.2.0/`fr265` compiles `["one", "two"]` into `apush`
referencing DATA+`0x329d` in `scratch/string-array-probe/`. The authored app prints
`a.toString()`, assigns `a[0] = "changed"`, and prints `a[0]`. Build with
`sh scripts/build-example.sh scratch/string-array-probe`; the probe uses the
shared console manifest/resources and an ordinary AppBase/onStart shell.

The 49-byte record has pool magic `0xabcdabcd`, pool length 12, then two local
strings at offsets 0 and 6. Each String has u16 length 4 including its terminator.
The value block has length 15, root `0x05`, count 2, and tag `0x03` references to
those pool offsets. Outer length is 43. Pool presence therefore does not imply
Dictionary; root-tag dispatch follows validated framing.

Structured CODE/DATA export reconstructs the PRG exactly and re-exports stably.
The reconstructed simulator output is `[one, two]`, then `changed`. Changing
only the first local pool text to `ONE`, assembling, and signing with our key
produces `[ONE, two]`, then `changed`. Artifacts: `structured/`, `reexport/`,
`edited/`, and `runtime-{rebuilt,edited}/` under the probe directory. This checks
String-pool editing and execution. Fresh repeated loads and the compiler pipeline
are covered by the [String-array specimen](compiler-string-arrays.md).
