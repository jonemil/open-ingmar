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
