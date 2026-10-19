# ByteArray encoding and runtime behavior

Reviewed and tested 2026-10-18 with SDK/compiler/simulator 9.2.0, fr265, `-O 0`.
The [authored fixture][fixture] and [expected console output][expected] cover
literal/dynamic construction, indexing, mutation, observers, and transforms.
SDK binaries and captures remain under `scratch/byte-array-probe/` and
`scratch/byte-array-fixture/`.

## Findings

**Officially documented:** [ByteArray][api] is a Lang.Object subclass introduced
at API 3.0.0. The API describes Number/Char byte arguments, mutation, comparison,
search, slicing, and reversal. Numeric encoding/decoding starts at 3.1.0 and
requires format/options contracts. These declarations do not establish our
compiler's executable support.

**Experimentally observed:** the SDK encodes nonempty constant literals with
`bpush` (`0x41`, u32 DATA address). Their shared constant wrapper contains root
`0x14`, u32 count, and untagged bytes. The literal
`[0, 1, 127, 128, 255, -1, -128]b` stores `00 01 7f 80 ff ff 80` and prints
unsigned Number elements. A write of `-1` reads back as `255`.

`new[n]b` consumes the Number length with `newba` (`0x36`, no inline operand).
Three slots start at zero. `[n, n + 1]b` allocates two slots with the same opcode
and fills them with `aputvdup`/`aputv` (`0x49`/`0x11`).

The fixture observes content equality between separate ByteArrays and inequality
against an ordinary Array. `add`/`addAll` mutate the shared receiver; Number
Arrays and ByteArrays both work as bulk inputs. Char `'A'` becomes byte `65`.
Search/removal, reversal, negative-end slicing, stable repeated hashing,
Method invocation, and live weak references produce the recorded output.
Only valid inputs and a live weak target were tested; range errors, expiry,
encoding options, and general exception behavior remain open.

Both the original SDK program and its structured reconstruction produce all
21 expected lines. Reconstruction is byte-identical, including this specimen's
signature. Editing only its first literal byte from `00` to `2a`, reassembling,
and signing changes only the first printed array's first element to `42`.
No CODE/DATA raw fallback is needed. These observations establish the binary
foundation, not frontend support or portable resource limits.

## Reproduction

Create `scratch/byte-array-fixture/monkey.jungle`:

```text
project.manifest=../../examples/minimal-console/manifest.xml
base.sourcePath=../../crates/ciq-dis/tests/fixtures/byte-arrays.mc
base.resourcePath=../../examples/minimal-console/resources
```

The manifest selects `MinimalConsoleApp` and fr265. Using the local SDK, device
files, and signing key, run:

```sh
sh scripts/build-example.sh scratch/byte-array-fixture
cargo run -p ciq-dis -- all \
  scratch/byte-array-fixture/build/fr265/byte-array-fixture.prg \
  scratch/byte-array-fixture/export
cargo run -p ciq-asm -- project scratch/byte-array-fixture/export \
  scratch/byte-array-fixture/rebuilt.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/byte-array-fixture/rebuilt.prg
```

Compare the complete console against the fixture's expected file. For the edit,
find `.bytes 00017f80ffff80` in exported DATA and replace its first byte with
`2a`; assemble/sign to a fresh output path and repeat. This changes no lengths
or addresses. Specimen offsets may differ with SDK/scaffold changes.

[fixture]: ../../crates/ciq-dis/tests/fixtures/byte-arrays.mc
[expected]: ../../crates/ciq-dis/tests/fixtures/byte-arrays.expected
[api]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang/ByteArray.html

## Source and IR profile

**Experimentally observed (2026-10-19):** the
[compiler example](../../examples/compiler-byte-arrays/README.md), using the
same SDK/target and `-O 0`, agrees with the independent compiler on all 149
expected lines. SDK/generated programs pass structured CODE/DATA reconstruction
and native/text pipeline checks. Artifacts:
`scratch/check-byte-array-complete/`. The full example requires minimum API 3.1.0.

The specimen allocates with `new[n]b`, writes Number and Char values, performs
an indexed compound, and observes unsigned readback. `addAll` accepts ByteArrays
and homogeneous Number/Char Arrays. Saved aliases retain the old object after
reassignment; writes through reverse/slice results leave the source unchanged.
Method invocation returns the current size; a live weak reference retrieves an
Object equal to the source. Conditional allocation prints only the selected
size helper, and a conditional length evaluates its chosen branch once.

Constant literals include signed bytes and ASCII Char; their emitted compact
payloads retain unsigned bytes. Repeated calls through one literal load return
independent mutable values, including empty constants. Computed literals use
private `newba` initialization with retaining/indexed stores. Both straight-line
and branching elements retain order: the branching example prints `10`, then
`11`, then `[10, 11, 90]`, without evaluating the unselected helper. A selected
literal arm likewise executes only its own element helper.

Nullable factory/echo calls preserve ByteArray-or-Null results. Early-return
guards permit indexed updates and non-null typed calls; Null takes the
`no bytes` branch. Short-circuit, conditional read, and loop guards produce the
expected values. Clearing the original nullable local leaves its weak target
alive through a saved alias; clearing that last alias immediately changes
`stillAlive()` from true to false on this simulator. No weak retrieval or
explicit collection is needed for this observation.

`ByteBox` initializes a non-null field from a parameter; a nullable field starts
as Null. An indexed field update is visible through a saved alias. Replacing
that field leaves the alias unchanged. Clearing a guarded snapshot leaves its
weak target alive through the nullable field; clearing the field expires it.
SDK/generated programs agree on these observations.

Arrays retain ByteArray elements by reference. An indexed element mutation is
visible through the original Array and its reversed copy. `slice` returns a
distinct Array (`equals` is false) while retaining element aliases; replacing
one copied element leaves the original slot unchanged. Add/bulk-add/remove
preserve the element references. Nullable/Null-only literals and sized allocation
produce the expected Null slots. Clearing an Array slot preserves a saved
snapshot. Clearing a snapshot first leaves its weak target alive through the
slot; clearing that slot expires it. Both compilers agree on these results.

String-keyed Dictionaries retain ByteArray values through construction and
`put`/indexed writes. Missing-key reads return Null. A guarded lookup mutation
reaches the stored object; replacing its entry leaves the saved alias unchanged.
`values()` preserves its own references after entries are removed. Null-valued
entries remain present according to `hasKey`. A nullable enumeration keeps a
weak target alive after its Dictionary entry is removed; replacing that
enumeration with an empty Array expires the target. The specimen does not rely
on key/value enumeration order. Both compilers agree on these observations.

An authored SDK probe tests an Object-or-Null parameter against
`Lang.ByteArray`. Its exported CODE uses `getmv @Toybox_Lang @ByteArray; isa`;
SYMBOLS identifies ByteArray as `0x008007a2`. Probe source/artifacts:
`scratch/byte-array-class/`; compile/export with the shared helpers.
The complete tracked example retains the tested class operand and equivalent
runtime recovery through `Bytes.recovered`.

Weak retrieval and bound `reverse` invocation return erased Object-or-Null
values. Positive ByteArray tests enable indexed updates and native calls on
those saved results; Array and Null arguments take the false branch. Non-null
and nullable `as ByteArray` assertions preserve valid references, and a nullable
assertion preserves Null. Assertions lower without a runtime check or conversion;
wrong-class assertion failure behavior was not executed.

**Officially documented (reviewed 2026-10-19):** [Lang constants and typedefs][lang]
define seven NumberFormat values, two Endian values, and Numeric's four scalar
alternatives. ByteArray's two numeric methods require API 3.1.0, with default
offset zero and little endian. Current value/option contracts are in the
[source specification](../specs/compiler-subset.md#bytearrays).

**Experimentally observed:** all seven formats round-trip with offset 1 and
big endian. FLOAT prints `1.500000` and tests as Float. UINT32 with
`4294967295l` decodes as Long; the smaller signed/unsigned cases test as Number.
Empty options encode `258` as `[2, 1, 0, 0]`; Double `2.5d` encodes/decodes as
binary32 `2.500000`. Numeric class guards permit arithmetic only on the
matching concrete result. Method selectors are decodeNumber `0x00800a25` and
encodeNumber `0x00800a26`; option symbols are offset `0x00800aa9` and endianness
`0x00800aae`, observed in exported authored SDK output.

The SDK rejects bare integer `4` as a NumberFormat argument and bare integer
`0` under `:endianness`, rather than treating them as enum values. The negative
probe uses `encodeNumber(258, 4, {:offset=>0, :endianness=>0})` and an equivalent
decode call. Diagnostics are retained in
`scratch/byte-final-raw-enums-sdk-build.log`. This establishes the separate enum
and named option-field checks; integer VM representation alone is insufficient
for the static contract.

ByteArray keys behave differently from direct `equals`: separate `[1, 2]b`
values compare equal and have equal hashes, but the second does not find the
first's Dictionary entry. Inserting both produces two entries. Mutating the
original key changes its public hash but its own lookup still finds the entry
in this simulator. A fresh equal-content key does not. These are target-scoped
observations; the compiler delegates comparison and mutation to the runtime.
`keys()` retains the original references after removal; dropping the final
enumeration expires a weak target. An isolated SDK probe under
`scratch/byte-array-final/` supplied the initial observations; tracked
`Bytes.codec`/`ByteProgram.finalChecks` retain their reproducible coverage.

Invalid byte ranges, fatal bounds errors, and invalid numeric options were not
executed here.
Negative/oversized constant inputs remain runtime obligations in compiler tests,
not observed portable exception guarantees.

For SDK comparison, use the minimal-console manifest/resources. The example
contains its own AppBase class. Set `base.sourcePath` to
`../../examples/compiler-byte-arrays` in
`scratch/compiler-byte-arrays/monkey.jungle`.
Build with `scripts/build-example.sh`, then use `scripts/check-example.py`
`prepare compiler-byte-arrays --sdk-prg <SDK PRG> --output <fresh directory>`
and `simulate <prepared directory>` in the Rust/SDK environments respectively.

[lang]: https://developer.garmin.com/connect-iq/api-docs/Toybox/Lang.html
