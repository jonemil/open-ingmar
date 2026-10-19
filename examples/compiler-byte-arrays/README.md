# ByteArray literals, allocation, and native methods

[app.mc](app.mc) exercises zero-filled allocation, Number/Char indexed writes,
compound updates, aliases, mutations, search, slicing, reversal, equality,
hashing, Method binding, and live weak references. Constant, empty, and computed
byte literals check fresh mutable results and ordered element effects.
Conditional elements/allocations check lazy branches and single evaluation.
Nullable calls, guarded indexing, short-circuit/loop checks, and weak expiry
exercise nullable reference flow and ownership through the last alias.
`ByteBox` checks initialized/non-null and default-null fields, shared mutation,
replacement, and field ownership of a weak target. The AppBase class delegates
startup to `ByteProgram.run`.
Arrays of ByteArrays check element aliases, shallow copies, nullable snapshots,
Null-only constants, sized allocation, mutation, and element ownership.
Typed Dictionaries check absent keys, guarded reads, value replacement,
nullable/Null-only entries, and ownership through `values()` enumerations.
Weak retrieval and Method results check runtime ByteArray refinement and
unchecked assertions from erased Object values.

API minimum: 3.0.0; target: 5.2.0; SDK/compiler/simulator: 9.2.0 / fr265.

From the repository root, using unused output paths:

```sh
mkdir -p scratch/compiler-byte-arrays
cargo run -p ciq-compiler -- --unit --emit prg \
  --target-api 5.2.0 --min-api 3.0.0 \
  --app-id a7086949531941379eb8b38164ed99aa --entry MinimalConsoleApp.onStart \
  examples/compiler-byte-arrays/app.mc scratch/compiler-byte-arrays/unsigned.prg
cargo run -p ciq-asm -- sign scratch/compiler-byte-arrays/unsigned.prg \
  scratch/compiler-byte-arrays/generated.prg -y keys/developer_key.der
sh scripts/run-simulator-container.sh \
  --prg scratch/compiler-byte-arrays/generated.prg
```

Compare the complete console with [expected.txt](expected.txt). See the
[source contract](../../docs/specs/compiler-subset.md#bytearrays) and
[SDK evidence](../../docs/investigations/byte-arrays.md).

ByteArray Dictionary keys and numeric encoding options remain
outside this example's supported profile.
